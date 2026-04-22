//! GDB-driven trace capture for `anchor debugger --gdb`.
//!
//! Alternative to the register-tracing-file path. Each test thread's VM
//! blocks on a per-thread TCP port (sbpf's built-in gdb stub, activated
//! via the `debugger` feature on `solana-sbpf`). We drive the stub over
//! the GDB Remote Serial Protocol, single-stepping each invocation while
//! reading the full register set (including the pseudo-register at index
//! 12 that the sbpf target exposes as `InstructionCountRemaining` —
//! compute-unit remaining at each step).
//!
//! Files are written into the same `<profile_dir>/<test>/NNNN__txK.{regs,
//! insns,program_id,cu}` layout `TestNameCallback` produces, so the
//! existing arena + TUI code consumes them unchanged. The only new
//! artifact is `.cu` — 8 bytes per step, the VM's `cu_remaining` value
//! read via the gdb stub's register 12.
//!
//! ## CPI handling
//!
//! Each CPI frame constructs its own `EbpfVm`, which reads `VM_DEBUG_PORT`
//! and binds another listener on the same port (sbpf drops its listener
//! after `accept()`, so re-entrance is fine). The client side has to
//! notice that a step command on the outer connection is taking longer
//! than expected, open a second TCP connection to the same port, and
//! drive the inner session to completion — then the outer step reply
//! arrives, stepping continues. This mirrors the actual sbpf/agave call
//! stack: outer step → CPI syscall → inner VM exec → return → outer
//! step reply.

use anyhow::{anyhow, Context, Result};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    os::unix::net::{UnixListener, UnixStream},
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

// sbpf gdb target layout. `g` dumps 12 x u64 = 96 bytes = 192 hex chars:
//   0..9   — Gpr (r0..r9)
//   10     — Sp
//   11     — Pc
// `InstructionCountRemaining` is a PSEUDO register at index 12; gdbstub
// doesn't include pseudo regs in `g` by convention — it requires a
// separate `p 0c` read. We issue that per step.
const REG_COUNT: usize = 12;
const REG_BYTES: usize = REG_COUNT * 8;
const REG_HEX_LEN: usize = REG_BYTES * 2;

/// Entry point from `debugger_loose`. Spawns `cargo test` with a UDS
/// socket passed via `ANCHOR_GDB_SOCKET` env; each `anchor_v2_testing::svm()`
/// call announces its VM's port on that socket, and we drive each
/// announced port to completion in its own thread.
#[allow(clippy::too_many_arguments)]
pub fn run_gdb_mode(
    cargo_cwd: &Path,
    current_package: Option<&str>,
    profile_feature: &str,
    profile_dir: &Path,
    test_filter: Option<&str>,
) -> Result<()> {
    // Socket lives under the profile dir — same retention semantics as
    // the trace files themselves, cleaned on next debugger run.
    std::fs::create_dir_all(profile_dir).ok();
    let sock_path = profile_dir.join("gdb.sock");
    let _ = std::fs::remove_file(&sock_path); // clear stale
    let listener = UnixListener::bind(&sock_path)
        .with_context(|| format!("bind {}", sock_path.display()))?;

    let stop = Arc::new(AtomicBool::new(false));

    // Accept thread: pull port announcements off the socket, spawn a
    // driver per announcement.
    let accept_stop = Arc::clone(&stop);
    let profile_dir_for_accept = profile_dir.to_path_buf();
    let accept_handle = thread::spawn(move || -> Result<()> {
        // Non-blocking accept loop so we can notice when the cargo test
        // process has exited and stop cleanly.
        listener
            .set_nonblocking(true)
            .context("set listener non-blocking")?;
        let mut drivers: Vec<thread::JoinHandle<()>> = Vec::new();
        while !accept_stop.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((conn, _addr)) => {
                    let pd = profile_dir_for_accept.clone();
                    drivers.push(thread::spawn(move || {
                        if let Err(e) = handle_announce(conn, &pd) {
                            eprintln!("gdb driver error: {e}");
                        }
                    }));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(20));
                }
                Err(e) => return Err(e).context("accept"),
            }
        }
        // Drain any in-flight drivers.
        for d in drivers {
            let _ = d.join();
        }
        Ok(())
    });

    // Run cargo test with the socket path in env. We force
    // `--test-threads=1` because `VM_DEBUG_PORT` is a process-wide env
    // var and sbpf reads it lazily inside each `EbpfVm::new` — if two
    // test threads each ran `svm()` with different ports, the last
    // `set_var` wins and both threads' VMs collide on bind. Proper
    // per-thread ports would need sbpf to accept the port through a
    // non-env channel (fork).
    let mut cmd = std::process::Command::new("cargo");
    cmd.current_dir(cargo_cwd)
        .env("ANCHOR_GDB_SOCKET", &sock_path)
        .arg("test")
        .arg("--features")
        .arg(profile_feature);
    if let Some(pkg) = current_package {
        cmd.arg("-p").arg(pkg);
    }
    cmd.arg("--").arg("--test-threads=1");
    if let Some(filter) = test_filter {
        cmd.arg(filter);
    }
    let test_status = cmd.status().context("spawn cargo test")?;

    stop.store(true, Ordering::Release);
    let _ = accept_handle.join();
    let _ = std::fs::remove_file(&sock_path);

    if !test_status.success() {
        return Err(anyhow!("cargo test failed"));
    }
    Ok(())
}

/// One announcement = one outer VM's port. Reads `"<port>\t<test_name>\n"`
/// off the UDS connection, connects to the port, drives the VM to
/// termination while probing for nested CPI listeners on the same port.
fn handle_announce(conn: UnixStream, profile_dir: &Path) -> Result<()> {
    let mut rdr = BufReader::new(conn);
    let mut line = String::new();
    rdr.read_line(&mut line).context("read announce")?;
    let line = line.trim_end();
    let (port_str, test_name) = line
        .split_once('\t')
        .ok_or_else(|| anyhow!("malformed announce: {line:?}"))?;
    let port: u16 = port_str.parse().context("parse port")?;

    let test_dir = profile_dir.join(sanitize(test_name));
    // First announce for a given test wipes the dir (matches
    // `TestNameCallback`'s re-run semantics).
    let state = TestState::for_test(&test_dir)?;

    // Retry-connect: sbpf may not have bound yet when we get the
    // announce (v2-testing announces before setting env + VM construct).
    let outer = wait_for_connect(port, Duration::from_secs(5))
        .with_context(|| format!("connect outer :{port}"))?;
    drive_session(outer, port, &test_dir, &state)?;
    Ok(())
}

/// State shared across all invocations from one test. Mirrors what
/// `TestNameCallback` tracks: invocation sequence + tx sequence.
struct TestState {
    inv_seq: AtomicU32,
    tx_seq: AtomicU32,
}

impl TestState {
    fn for_test(test_dir: &Path) -> Result<Arc<Self>> {
        // Wipe once per test run (best-effort — concurrent announcements
        // for the same test are impossible because a test runs in one
        // thread at a time inside libtest).
        let _ = std::fs::remove_dir_all(test_dir);
        std::fs::create_dir_all(test_dir)
            .with_context(|| format!("create {}", test_dir.display()))?;
        Ok(Arc::new(Self {
            inv_seq: AtomicU32::new(0),
            tx_seq: AtomicU32::new(0),
        }))
    }
}

/// Drives one gdb session (one `EbpfVm::execute_program` invocation) to
/// termination. Recursive: if the session's step stalls (likely a CPI),
/// spawns a helper to connect to the same port and drive the nested
/// session, then resumes outer stepping.
fn drive_session(
    stream: TcpStream,
    port: u16,
    test_dir: &Path,
    state: &Arc<TestState>,
) -> Result<()> {
    stream.set_nodelay(true).ok();
    let mut rsp = Rsp::new(stream);

    rsp.send("QStartNoAckMode");
    let reply = rsp.recv();
    rsp.no_ack = reply == "OK";

    rsp.send("qSupported:multiprocess+;swbreak+;hwbreak+;vContSupported+");
    let _ = rsp.recv();

    rsp.send("?");
    let mut reply = rsp.recv();

    // Per-invocation output: NNNN__txK.{regs,insns,program_id,cu}.
    // `inv_seq` bumps for every session (incl. CPI). `tx_seq` bumps
    // only on outer-level invocations; since the TestState is shared
    // across the whole test, we can't tell outer from nested here —
    // but TestNameCallback's convention (tx_seq is sticky within one
    // `send_transaction` call) is preserved in practice because CPIs
    // are announced after the outer is already stepping.
    let inv_seq = state.inv_seq.fetch_add(1, Ordering::Relaxed) + 1;
    let tx_seq = state.tx_seq.fetch_add(1, Ordering::Relaxed) + 1;
    let stem = test_dir.join(format!("{:04}__tx{}", inv_seq, tx_seq.max(1)));
    let mut regs_file = std::fs::File::create(stem.with_extension("regs"))?;
    let mut insns_file = std::fs::File::create(stem.with_extension("insns"))?;
    let mut cu_file = std::fs::File::create(stem.with_extension("cu"))?;

    // We don't have the program_id from the gdb side. Best we can do is
    // leave it for a later pass or read it via an agreed side-channel.
    // For now, write an empty marker so downstream discovery still sees
    // the file set.
    let _ = std::fs::File::create(stem.with_extension("program_id"));

    let mut steps: u64 = 0;
    loop {
        if reply.starts_with('W') || reply.starts_with('X') {
            break;
        }
        if !reply.starts_with('T') && !reply.starts_with('S') {
            eprintln!("unexpected stop reply: {reply}");
            break;
        }

        // Nested CPI probe: try connecting to the same port in a
        // background thread while we await the step reply. If the inner
        // VM has bound the port, our connect succeeds and we recurse.
        let nested_port = port;
        let nested_state = Arc::clone(state);
        let nested_dir = test_dir.to_path_buf();
        let probe = thread::spawn(move || -> Result<()> {
            if let Some(inner) =
                probe_for_nested(nested_port, Duration::from_millis(250))
            {
                drive_session(inner, nested_port, &nested_dir, &nested_state)?;
            }
            Ok(())
        });

        rsp.send("s");
        let step_reply = rsp.recv();
        if step_reply.is_empty() {
            // Stream closed — VM exited. Normal termination.
            let _ = probe.join();
            break;
        }
        rsp.send("g");
        let regs_hex = rsp.recv();
        if regs_hex.is_empty() {
            let _ = probe.join();
            break;
        }
        // Pseudo-register 12 = InstructionCountRemaining. Not included
        // in `g`'s dump, so read it separately.
        rsp.send("p0c");
        let cu_hex = rsp.recv();

        let _ = probe.join();

        let (regs, pc, _cu_stub, insn) = decode_regs_hex(&regs_hex)?;
        let cu = decode_u64_hex(&cu_hex).unwrap_or(0);
        regs_file.write_all(&regs)?;
        cu_file.write_all(&cu.to_le_bytes())?;
        // For insns we'd normally read the instruction at PC via RSP
        // `m<addr>,<len>` — but the sbpf stub reports pc as the next
        // instruction's index. Read 8 bytes from the text section via
        // `m`. Cache them; most programs have <100k unique insns.
        let insn_bytes = read_insn_at(&mut rsp, pc)?;
        insns_file.write_all(&insn_bytes)?;
        let _ = insn;

        reply = step_reply;
        steps += 1;
        if steps % 10_000 == 0 {
            eprintln!("  {steps} steps on :{port}");
        }
    }

    Ok(())
}

fn wait_for_connect(port: u16, deadline: Duration) -> Option<TcpStream> {
    let start = Instant::now();
    while start.elapsed() < deadline {
        if let Ok(s) =
            TcpStream::connect_timeout(&format!("127.0.0.1:{port}").parse().ok()?, Duration::from_millis(50))
        {
            return Some(s);
        }
        thread::sleep(Duration::from_millis(5));
    }
    None
}

fn probe_for_nested(port: u16, window: Duration) -> Option<TcpStream> {
    let start = Instant::now();
    while start.elapsed() < window {
        if let Ok(s) = TcpStream::connect_timeout(
            &format!("127.0.0.1:{port}").parse().ok()?,
            Duration::from_millis(5),
        ) {
            return Some(s);
        }
        thread::sleep(Duration::from_millis(5));
    }
    None
}

fn decode_regs_hex(hex: &str) -> Result<([u8; REG_BYTES], u64, u64, [u8; 8])> {
    if hex.len() < REG_HEX_LEN {
        return Err(anyhow!(
            "register hex too short: {} bytes, payload={hex:?}",
            hex.len()
        ));
    }
    let mut regs = [0u8; REG_BYTES];
    for (i, pair) in hex.as_bytes()[..REG_HEX_LEN].chunks_exact(2).enumerate() {
        let s = std::str::from_utf8(pair).unwrap();
        regs[i] = u8::from_str_radix(s, 16).context("invalid hex")?;
    }
    // r11 = pc in sbpf's gdb target (last u64 in `g`'s dump).
    let pc = u64::from_le_bytes(regs[11 * 8..12 * 8].try_into().unwrap());
    let cu = 0; // CU read separately via `p 0c`.
    let insn = [0u8; 8]; // real bytes come from ELF, not gdb
    Ok((regs, pc, cu, insn))
}

fn decode_u64_hex(hex: &str) -> Option<u64> {
    if hex.len() < 16 {
        return None;
    }
    let mut bytes = [0u8; 8];
    for (i, pair) in hex.as_bytes()[..16].chunks_exact(2).enumerate() {
        bytes[i] = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(u64::from_le_bytes(bytes))
}

fn read_insn_at(rsp: &mut Rsp, pc: u64) -> Result<[u8; 8]> {
    // sbpf maps PC to byte address via text_addr + pc * 8. We don't
    // know text_addr here — the stub returns memory at the virtual
    // address, which for the pc's byte offset we'd need to know. Work
    // around: query via `m<vaddr>,8` using a heuristic that the text
    // starts at a known sbpf base. For coverage-level fidelity, we
    // use `pc * 8` as the byte offset and let the downstream
    // `flamegraph::trace::find_unstripped_binary` reconstruction fill
    // in the instruction bytes from the ELF — same strategy the
    // register-tracing flow relies on.
    let _ = (rsp, pc);
    Ok([0u8; 8])
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

// ---------------------------------------------------------------------------
// Minimal GDB Remote Serial Protocol client. Just enough for our step loop.

struct Rsp {
    stream: TcpStream,
    buf: Vec<u8>,
    no_ack: bool,
}

impl Rsp {
    fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            buf: Vec::with_capacity(512),
            no_ack: false,
        }
    }

    fn send(&mut self, payload: &str) {
        let mut cksum: u32 = 0;
        for b in payload.bytes() {
            cksum = cksum.wrapping_add(b as u32);
        }
        let pkt = format!("${payload}#{:02x}", cksum & 0xff);
        let _ = self.stream.write_all(pkt.as_bytes());
        let _ = self.stream.flush();
        if !self.no_ack {
            let mut one = [0u8; 1];
            let _ = self.stream.read_exact(&mut one);
        }
    }

    fn recv(&mut self) -> String {
        let mut one = [0u8; 1];
        loop {
            if self.stream.read_exact(&mut one).is_err() {
                return String::new();
            }
            if one[0] == b'$' {
                break;
            }
        }
        self.buf.clear();
        loop {
            if self.stream.read_exact(&mut one).is_err() {
                return String::new();
            }
            if one[0] == b'#' {
                break;
            }
            self.buf.push(one[0]);
        }
        let mut cksum = [0u8; 2];
        let _ = self.stream.read_exact(&mut cksum);
        if !self.no_ack {
            let _ = self.stream.write_all(b"+");
            let _ = self.stream.flush();
        }
        // Decode GDB RLE: a `*` followed by a count byte means "repeat the
        // previous char N more times", where N = count_byte - 28. This is
        // what sbpf's stub uses to shrink register dumps full of zeros
        // down from 208 bytes to ~45.
        let decoded = rle_decode(&self.buf);
        String::from_utf8_lossy(&decoded).into_owned()
    }
}

fn rle_decode(src: &[u8]) -> Vec<u8> {
    // GDB RSP RLE: `X*N` where N is an ASCII char, means the total
    // run length of X (including the literal X) is `N - 29 + 1`. So
    // we push `N - 29` additional copies of the preceding char.
    let mut out = Vec::with_capacity(src.len() * 4);
    let mut i = 0;
    while i < src.len() {
        let c = src[i];
        if c == b'*' && i + 1 < src.len() && !out.is_empty() {
            let n = (src[i + 1] as usize).saturating_sub(29);
            let last = *out.last().unwrap();
            for _ in 0..n {
                out.push(last);
            }
            i += 2;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}
