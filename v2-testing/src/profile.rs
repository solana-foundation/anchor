//! Per-test register-trace capture.
//!
//! Behind the `profile` feature, `svm()` returns a `LiteSVM` with
//! register tracing turned on and a custom [`InvocationInspectCallback`]
//! that routes trace files into a per-test directory keyed on
//! `std::thread::current().name()`. Multi-tx tests (and their CPIs) all
//! land under the same test directory so downstream tooling produces a
//! single flamegraph per test.
//!
//! ## File layout
//!
//! ```text
//! target/anchor-v2-profile/
//! └── <sanitized_test_name>/
//!     ├── 0001__tx1.regs          ← tx 1, top-level invocation
//!     ├── 0001__tx1.insns
//!     ├── 0001__tx1.program_id
//!     ├── 0002__tx1.regs          ← tx 1, CPI into another program
//!     ├── 0002__tx1.insns
//!     ├── 0002__tx1.program_id
//!     ├── 0003__tx2.regs          ← tx 2
//!     └── ...
//! ```
//!
//! - `NNNN` is a monotonic invocation counter (CPIs get their own index).
//! - `txN` identifies the outer transaction — bumped once per
//!   `send_transaction`.
//! - Both counters are per-test, reset across processes.
//!
//! ## Re-run semantics
//!
//! The first invocation that fires for a given `(process, test)` pair
//! wipes the test's directory before writing. This guarantees a test
//! that previously produced more invocations than its current version
//! doesn't leak stale traces into a subsequent run.
//!
//! Override the root directory with the `ANCHOR_PROFILE_DIR` env var.

use {
    litesvm::{InvocationInspectCallback, LiteSVM},
    solana_program_runtime::invoke_context::{Executable, InvokeContext, RegisterTrace},
    solana_transaction::sanitized::SanitizedTransaction,
    solana_transaction_context::{IndexOfAccount, InstructionContext},
    std::{
        collections::HashMap,
        fs::{self, File},
        io::Write,
        path::PathBuf,
        sync::Mutex,
    },
};

/// Reinterpret a slice of POD values as raw bytes. litesvm's own
/// `register_tracing::as_bytes` is `pub(crate)` so we replicate it here.
fn as_bytes<T>(slice: &[T]) -> &[u8] {
    // Safety: T is a fixed-size POD (in practice `[u64; 12]`) — reinterpreting
    // its byte image is a well-defined operation on stable Rust so long as
    // we don't write through it.
    unsafe {
        std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice))
    }
}

const DEFAULT_DIR: &str = "target/anchor-v2-profile";

/// Construct a `LiteSVM` wired up for Anchor v2 testing.
///
/// With the `profile` feature, installs [`TestNameCallback`] so each
/// transaction writes an SBF register trace under
/// `target/anchor-v2-profile/<test_name>/`. Without it, identical to
/// [`LiteSVM::new()`].
pub fn svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    svm.set_invocation_inspect_callback(TestNameCallback::new());
    svm
}

struct TestState {
    /// Monotonic counter across all invocations (including CPIs) in this test.
    inv_seq: u32,
    /// Monotonic counter across outer transactions in this test. Bumped in
    /// `before_invocation`, which only fires at tx top level.
    tx_seq: u32,
    /// True once this test's directory has been wiped and recreated.
    cleaned: bool,
}

/// Register-tracing callback that keys trace files by test name.
///
/// Implements [`InvocationInspectCallback`]. Use via
/// [`LiteSVM::set_invocation_inspect_callback`].
pub struct TestNameCallback {
    root: PathBuf,
    state: Mutex<HashMap<String, TestState>>,
}

impl TestNameCallback {
    pub fn new() -> Self {
        let root = std::env::var("ANCHOR_PROFILE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_DIR));
        Self {
            root,
            state: Mutex::new(HashMap::new()),
        }
    }

    fn test_name() -> String {
        std::thread::current()
            .name()
            .unwrap_or("unknown")
            .replace("::", "__")
    }

    fn test_dir(&self, test: &str) -> PathBuf {
        self.root.join(test)
    }

    /// Bumps the tx counter (called from `before_invocation`, so only
    /// outer tx entry — CPIs don't go through this path) and returns
    /// the current tx number.
    fn bump_tx_seq(&self, test: &str) -> u32 {
        let mut state = self.state.lock().unwrap();
        let entry = state.entry(test.to_owned()).or_insert(TestState {
            inv_seq: 0,
            tx_seq: 0,
            cleaned: false,
        });
        entry.tx_seq += 1;
        entry.tx_seq
    }

    /// Bumps the invocation counter and returns `(inv_seq, tx_seq,
    /// should_clean)`. `should_clean` is true on the first call for a
    /// test in this process — caller is responsible for wiping the
    /// test's directory before writing.
    fn bump_inv_seq(&self, test: &str) -> (u32, u32, bool) {
        let mut state = self.state.lock().unwrap();
        let entry = state.entry(test.to_owned()).or_insert(TestState {
            inv_seq: 0,
            tx_seq: 0,
            cleaned: false,
        });
        entry.inv_seq += 1;
        let should_clean = !entry.cleaned;
        entry.cleaned = true;
        (entry.inv_seq, entry.tx_seq.max(1), should_clean)
    }
}

impl Default for TestNameCallback {
    fn default() -> Self {
        Self::new()
    }
}

impl InvocationInspectCallback for TestNameCallback {
    fn before_invocation(
        &self,
        _: &LiteSVM,
        _: &SanitizedTransaction,
        _: &[IndexOfAccount],
        _: &InvokeContext,
    ) {
        let test = Self::test_name();
        self.bump_tx_seq(&test);
    }

    fn after_invocation(
        &self,
        _svm: &LiteSVM,
        invoke_context: &InvokeContext,
        register_tracing_enabled: bool,
    ) {
        if !register_tracing_enabled {
            return;
        }

        let test = Self::test_name();
        let dir = self.test_dir(&test);

        invoke_context.iterate_vm_traces(
            &|ictx: InstructionContext, _exec: &Executable, trace: RegisterTrace| {
                if trace.is_empty() {
                    return;
                }

                let (inv_seq, tx_seq, should_clean) = self.bump_inv_seq(&test);

                // Wipe on the first invocation we write for this test so
                // stale traces from a longer prior run can't leak through.
                // `bump_inv_seq` guarantees `should_clean == true` only once.
                if should_clean {
                    let _ = fs::remove_dir_all(&dir);
                }
                if fs::create_dir_all(&dir).is_err() {
                    return;
                }

                let stem = dir.join(format!("{inv_seq:04}__tx{tx_seq}"));

                // .regs — raw [u64; 12] register states. PC is r11.
                if let Ok(mut f) = File::create(stem.with_extension("regs")) {
                    for regs in trace.iter() {
                        let _ = f.write_all(as_bytes(regs.as_slice()));
                    }
                }
                // .program_id — base58 program invoked for this trace.
                if let Ok(pid) = ictx.get_program_key() {
                    if let Ok(mut f) = File::create(stem.with_extension("program_id")) {
                        let _ = write!(f, "{}", pid);
                    }
                }
            },
        );
    }
}
