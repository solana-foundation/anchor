use anyhow::{anyhow, Context, Result};
use object::{Object, ObjectSection, ObjectSymbol, SymbolKind};
use rustc_demangle::demangle;
use solana_sbpf::{
    ebpf,
    elf::Executable,
    program::BuiltinProgram,
    static_analysis::Analysis,
    vm::{Config, ContextObject},
};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

pub struct FlamegraphReport {
    pub program_name: String,
    pub total_cu: u64,
    pub stacks: BTreeMap<Vec<String>, u64>,
}

/// Size in bytes of one register trace entry: 12 x u64 = 96 bytes.
pub(super) const REGS_ENTRY_SIZE: usize = 12 * std::mem::size_of::<u64>();

/// Size in bytes of one raw SBPF instruction: 8 bytes.
pub(super) const INSN_ENTRY_SIZE: usize = 8;

/// Provides the minimal VM context needed to parse and inspect an executable.
#[derive(Default)]
struct NoopContext;

impl ContextObject for NoopContext {
    fn consume(&mut self, _amount: u64) {}
    fn get_remaining(&self) -> u64 {
        0
    }
}

/// Builds a flamegraph report from LiteSVM register trace files and the program ELF.
///
/// The trace directory should contain `.regs` and `.insns` files produced by
/// `DefaultRegisterTracingCallback`. The ELF is used to resolve PCs to function names.
pub fn build_report_from_trace(
    program_name: &str,
    elf_path: &Path,
    trace_dir: &Path,
    manifest_dir: Option<&Path>,
) -> Result<Option<FlamegraphReport>> {
    let (symbol_map, syscall_names) = load_function_map(elf_path, manifest_dir)?;

    // Find the trace files in the directory. There may be multiple sets (one per
    // invocation); we combine them all.
    let trace_files = find_trace_files(trace_dir)?;
    if trace_files.is_empty() {
        eprintln!(
            "Warning: No trace files found in {} for `{program_name}`. Skipping flamegraph.",
            trace_dir.display()
        );
        return Ok(None);
    }

    let mut folded_stacks: BTreeMap<Vec<String>, u64> = BTreeMap::new();
    let mut total_cu: u64 = 0;

    for (regs_path, insns_path) in &trace_files {
        let regs_data = fs::read(regs_path)
            .with_context(|| format!("Failed to read {}", regs_path.display()))?;
        let insns_data = fs::read(insns_path)
            .with_context(|| format!("Failed to read {}", insns_path.display()))?;

        let entry_count = regs_data.len() / REGS_ENTRY_SIZE;
        let insn_count = insns_data.len() / INSN_ENTRY_SIZE;
        let count = entry_count.min(insn_count);

        if count == 0 {
            continue;
        }

        let (stacks, cu) =
            process_trace(&regs_data, &insns_data, count, &symbol_map, &syscall_names, program_name);
        for (stack, cost) in stacks {
            *folded_stacks.entry(stack).or_default() += cost;
        }
        total_cu += cu;
    }

    if total_cu == 0 {
        return Ok(None);
    }

    Ok(Some(FlamegraphReport {
        program_name: program_name.to_owned(),
        total_cu,
        stacks: folded_stacks,
    }))
}

/// Processes a single trace (regs + insns pair) and returns folded stacks + total CU.
///
/// The algorithm is PC-driven rather than opcode-driven: for every traced
/// instruction we look up which function the PC falls into and resync the
/// maintained call stack against it. If the resolved function is already in
/// the stack we pop down to it (we missed one or more returns); if it's not
/// in the stack we push it (we missed a call, e.g. via a tail call or direct
/// branch). This is more robust than dispatching off `EXIT`/`RETURN` opcodes
/// because direct jumps between functions never produce a matching call/ret
/// pair.
///
/// The one case we still special-case is SBPFv1's `CALL_IMM` → syscall: a
/// syscall does not change the persistent call stack (the caller resumes at
/// the next instruction), but we still want to attribute its single traced
/// CU to a `[syscall] {name}` leaf frame for display.
fn process_trace(
    regs_data: &[u8],
    insns_data: &[u8],
    count: usize,
    symbol_map: &BTreeMap<u64, String>,
    syscall_names: &BTreeMap<u32, String>,
    program_name: &str,
) -> (BTreeMap<Vec<String>, u64>, u64) {
    let mut folded_stacks: BTreeMap<Vec<String>, u64> = BTreeMap::new();
    let mut total_cu: u64 = 0;

    // Track the call stack by maintaining a stack of function display names.
    // The root frame is the program name and is never popped. Every other
    // frame is formatted as `{function_name} @ {entry_pc:#x}` so the SVG
    // shows both the resolved symbol and its SBPF entry point.
    let mut call_stack: Vec<String> = vec![program_name.to_owned()];

    for i in 0..count {
        let regs = read_regs(regs_data, i);
        let insn = read_insn(insns_data, i);
        let pc = regs[11];

        // Resolve the function containing this PC along with its entry PC,
        // and build the display frame name we'd use if we had to push it.
        let (current_name, current_entry_pc) = lookup_function_with_pc(symbol_map, pc);
        let current_frame = format!("{current_name} @ {current_entry_pc:#x}");

        // PC-driven call-stack resync. If the top of the stack no longer
        // matches the current frame, either we missed one or more returns
        // (pop down to `current_frame` if it is anywhere in the stack) or we
        // missed a call (push `current_frame`).
        if call_stack.last().map(String::as_str) != Some(current_frame.as_str()) {
            if let Some(depth) = call_stack.iter().rposition(|f| f == &current_frame) {
                call_stack.truncate(depth + 1);
            } else {
                call_stack.push(current_frame);
            }
        }

        // SBPFv1 CALL_IMM is overloaded: it's either an internal call (PC
        // jumps to the callee on the next trace entry) or an external
        // syscall (PC advances by 1). We only need to intercept the syscall
        // case — internal calls are handled by the resync above on the next
        // iteration, which will see the callee's PC and push it.
        if insn[0] == ebpf::CALL_IMM {
            let imm = u32::from_le_bytes(insn[4..8].try_into().unwrap());
            let is_syscall = if i + 1 < count {
                let next_regs = read_regs(regs_data, i + 1);
                let next_pc = next_regs[11];
                // Syscall: PC advances by 1 (next sequential instruction).
                // Internal call: PC jumps to a different location.
                next_pc == pc + 1
            } else {
                // Last traced instruction — treat as internal call so the
                // call site is attributed to the current frame rather than
                // an unresolvable syscall stub.
                false
            };

            if is_syscall {
                let syscall_name = syscall_names
                    .get(&imm)
                    .cloned()
                    .unwrap_or_else(|| format!("syscall_{imm:#x}"));
                let mut syscall_stack = call_stack.clone();
                syscall_stack.push(format!("[syscall] {syscall_name}"));
                *folded_stacks.entry(syscall_stack).or_default() += 1;
                total_cu += 1;
                continue;
            }
        }

        // Default attribution: credit one CU to the current call stack top.
        // EXIT / RETURN opcodes fall into this arm as well — their pop is
        // handled implicitly by the next iteration's PC-driven resync once
        // control returns to the caller.
        *folded_stacks.entry(call_stack.clone()).or_default() += 1;
        total_cu += 1;
    }

    (folded_stacks, total_cu)
}

/// Reads the i-th register entry (12 x u64) from raw bytes.
pub(super) fn read_regs(data: &[u8], i: usize) -> [u64; 12] {
    let offset = i * REGS_ENTRY_SIZE;
    let mut regs = [0u64; 12];
    for r in 0..12 {
        let start = offset + r * 8;
        let bytes: [u8; 8] = data[start..start + 8].try_into().unwrap();
        regs[r] = u64::from_le_bytes(bytes);
    }
    regs
}

/// Reads the i-th instruction entry (8 bytes) from raw bytes.
pub(super) fn read_insn(data: &[u8], i: usize) -> [u8; 8] {
    let offset = i * INSN_ENTRY_SIZE;
    data[offset..offset + 8].try_into().unwrap()
}

/// Looks up the function name AND entry PC for a given PC using the symbol
/// map, by walking to the nearest lower-or-equal symbol entry. Returns
/// `("unknown_{pc:#x}", pc)` as a fallback when the map has no covering
/// entry.
pub(super) fn lookup_function_with_pc(symbol_map: &BTreeMap<u64, String>, pc: u64) -> (String, u64) {
    symbol_map
        .range(..=pc)
        .next_back()
        .map(|(entry_pc, name)| (name.clone(), *entry_pc))
        .unwrap_or_else(|| (format!("unknown_{pc:#x}"), pc))
}

/// Finds all (*.regs, *.insns) file pairs in the trace directory.
pub(super) fn find_trace_files(trace_dir: &Path) -> Result<Vec<(std::path::PathBuf, std::path::PathBuf)>> {
    let mut pairs = Vec::new();

    if !trace_dir.exists() {
        return Ok(pairs);
    }

    let entries = fs::read_dir(trace_dir)
        .with_context(|| format!("Failed to read trace directory {}", trace_dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("regs") {
            let insns_path = path.with_extension("insns");
            if insns_path.exists() {
                pairs.push((path, insns_path));
            }
        }
    }

    Ok(pairs)
}

/// Loads function symbols from an ELF file using the SBPF loader.
///
/// This parses the ELF with `solana_sbpf` to extract the function registry and
/// analysis labels, which contain all internal function entry points with their
/// demangled names. Additionally loads symbols from the ELF's object-level symbol
/// table (dynamic symbols) as a fallback.
///
/// If the deployed binary is stripped (common for `cargo-build-sbf`), we also
/// try loading symbols from the unstripped build artifact in the
/// `target/sbpf-solana-solana/release/` directory.
pub(super) fn load_function_map(
    elf_path: &Path,
    manifest_dir: Option<&Path>,
) -> Result<(BTreeMap<u64, String>, BTreeMap<u32, String>)> {
    let elf_bytes = fs::read(elf_path)
        .with_context(|| format!("Failed to read ELF {}", elf_path.display()))?;

    // Parse through SBPF to get function registry labels.
    let loader = Arc::new(BuiltinProgram::new_loader(Config {
        enable_symbol_and_section_labels: true,
        ..Config::default()
    }));

    let executable = Executable::<NoopContext>::from_elf(&elf_bytes, loader)
        .map_err(|err| anyhow!("Failed to parse SBPF ELF {}: {err}", elf_path.display()))?;

    let analysis = Analysis::from_executable(&executable)
        .map_err(|err| anyhow!("Failed to analyze SBPF executable: {err}"))?;

    let mut symbols: BTreeMap<u64, String> = BTreeMap::new();

    // Primary source: SBPF analysis function labels (internal function registry).
    for (pc, (_key, name)) in analysis.functions.iter() {
        let normalized = normalize_symbol(name, *pc);
        symbols.entry(*pc as u64).or_insert(normalized);
    }

    // Secondary source: symbols from the deployed ELF (often stripped).
    if let Ok(extra) = load_elf_symbols(&elf_bytes) {
        for (pc, name) in extra {
            symbols.entry(pc as u64).or_insert(name);
        }
    }

    // Tertiary source: the unstripped pre-deploy binary in the build directory.
    // cargo-build-sbf strips the binary before copying to target/deploy/, but
    // the unstripped version remains in target/sbpf-solana-solana/release/.
    if let Some(unstripped_path) = find_unstripped_binary(elf_path, manifest_dir) {
        if let Ok(unstripped_bytes) = fs::read(&unstripped_path) {
            if let Ok(extra) = load_elf_symbols(&unstripped_bytes) {
                for (pc, name) in extra {
                    // Only overwrite generic "function_N" labels.
                    let entry = symbols.entry(pc as u64);
                    match entry {
                        std::collections::btree_map::Entry::Vacant(v) => {
                            v.insert(name);
                        }
                        std::collections::btree_map::Entry::Occupied(mut o) => {
                            if o.get().starts_with("function_") {
                                o.insert(name);
                            }
                        }
                    }
                }
            }
        }
    }

    // Syscall name map: Murmur3 hash → name.
    // These are the standard Solana runtime syscalls.
    let syscall_names: BTreeMap<u32, String> = [
        (0xb6fc1a11, "abort"),
        (0x207559bd, "sol_log_"),
        (0x5c2a3178, "sol_log_64_"),
        (0x52ba5096, "sol_log_compute_units_"),
        (0x7317b434, "sol_log_data"),
        (0x11f49d86, "sol_sha256"),
        (0xd7793abb, "sol_keccak256"),
        (0x174c5122, "sol_blake3"),
        (0xaa2607ca, "sol_curve_validate_point"),
        (0xdd1c41a6, "sol_curve_group_op"),
        (0xa22b9c85, "sol_invoke_signed_c"),
        (0xd7449092, "sol_invoke_signed_rust"),
        (0xa226d3eb, "sol_set_return_data"),
        (0x5d2245e4, "sol_get_return_data"),
        (0x9377323c, "sol_create_program_address"),
        (0x48504a38, "sol_try_find_program_address"),
        (0x717cc4a3, "sol_memcpy_"),
        (0x434371f8, "sol_memmove_"),
        (0x3770fb22, "sol_memset_"),
        (0x5fdcde31, "sol_memcmp_"),
    ].into_iter().map(|(k, v)| (k, v.to_owned())).collect();

    Ok((symbols, syscall_names))
}

/// Tries to locate the unstripped SBF binary for a deployed program.
///
/// `cargo-build-sbf --sbf-out-dir <dir>` copies a **stripped** .so into
/// `<dir>/`, but the unstripped build artifact remains in the cargo target
/// tree of whichever workspace actually compiled the program. In the bench
/// setup that can be any of:
///
///   - `<bench>/target/sbpf-solana-solana/release/<name>.so` — for programs
///     that are bench-workspace members (anchor v1 / v2).
///   - `<bench>/programs/<family>/<variant>/target/sbpf-solana-solana/release/<name>.so`
///     — for programs with their own `[workspace]` (pinocchio / steel / quasar).
///   - `<repo>/target/sbpf-solana-solana/release/<name>.so` — historical
///     location when building from the repo root.
///
/// Rather than enumerate every path combinatorially, we walk upward from the
/// deployed .so until we hit a directory that contains a `bench/` or
/// `programs/` sibling (the repo root-ish), then do a bounded recursive
/// search under it for a file matching `name` inside any
/// `sbpf-solana-solana/release` directory. Returns the first non-stripped
/// match, or `None` if nothing is found.
fn find_unstripped_binary(
    deployed_path: &Path,
    manifest_dir: Option<&Path>,
) -> Option<std::path::PathBuf> {
    let file_name = deployed_path.file_name()?.to_str()?.to_owned();

    // Preferred path: walk up from the manifest dir to find the nearest
    // containing `[workspace]` Cargo.toml (the workspace root that actually
    // built the program). For an isolated `[workspace]` program the root is
    // the manifest dir itself; for a bench-workspace member it's several
    // levels up (e.g. `bench/programs/helloworld/anchor-v1` → `bench/`).
    // Cargo always places build artifacts under `<workspace_root>/target/`,
    // so this gives us a precise lookup with no ambiguity — the same
    // program rebuilt at a different lib name couldn't leak a stale binary
    // because the workspace root is deterministic from the manifest path.
    if let Some(manifest) = manifest_dir {
        if let Some(root) = find_workspace_root(manifest) {
            let direct = root
                .join("target")
                .join("sbpf-solana-solana")
                .join("release")
                .join(&file_name);
            if direct.exists() {
                return Some(direct);
            }
            let deps = root
                .join("target")
                .join("sbpf-solana-solana")
                .join("release")
                .join("deps")
                .join(&file_name);
            if deps.exists() {
                return Some(deps);
            }
        }
    }

    // Fallback: walk up the deployed path to a plausible repo root and
    // recursively search for the file under any `sbpf-solana-solana/release`
    // directory. This handles historical layouts and cases where no
    // manifest_dir was supplied.
    let mut root = deployed_path.parent()?;
    loop {
        let parent = root.parent()?;
        let has_bench = parent.join("bench").is_dir();
        let has_target = parent.join("target").is_dir();
        if has_bench || has_target {
            root = parent;
            break;
        }
        root = parent;
    }

    search_for_unstripped(root, &file_name, 0)
}

/// Walks up from `manifest_dir` looking for a `Cargo.toml` that declares a
/// `[workspace]` table. Returns the directory of the first such manifest, or
/// `manifest_dir` itself if none is found (assumes single-crate repo).
fn find_workspace_root(manifest_dir: &Path) -> Option<std::path::PathBuf> {
    let mut current: std::path::PathBuf = manifest_dir.to_path_buf();
    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(contents) = fs::read_to_string(&cargo_toml) {
                // Crude but good enough: the marker `[workspace]` at the
                // start of a line or after a newline means this Cargo.toml
                // defines a workspace.
                if contents.contains("\n[workspace]") || contents.starts_with("[workspace]") {
                    return Some(current);
                }
            }
        }
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return None,
        }
    }
}

/// Recursively searches `dir` for `file_name` inside any
/// `sbpf-solana-solana/release` subdirectory. Depth-limited to avoid
/// pathological descent into dependencies. Returns the first match that is
/// not stripped (larger than the matching deployed .so would be — we can't
/// cheaply check strip state, but filtering by "inside release" usually
/// works since cargo's own target tree always keeps symbols).
fn search_for_unstripped(
    dir: &Path,
    file_name: &str,
    depth: usize,
) -> Option<std::path::PathBuf> {
    // Hard cap on recursion: repo layouts never nest workspaces more than
    // ~4 deep (e.g. `<repo>/bench/programs/<family>/<variant>/target/...`).
    // 6 is generous and cheap.
    if depth > 6 {
        return None;
    }

    // Quick check: does this directory already contain the file we want at
    // the expected `sbpf-solana-solana/release/<file>` path?
    let direct = dir
        .join("target")
        .join("sbpf-solana-solana")
        .join("release")
        .join(file_name);
    if direct.exists() {
        return Some(direct);
    }
    let deps = dir
        .join("target")
        .join("sbpf-solana-solana")
        .join("release")
        .join("deps")
        .join(file_name);
    if deps.exists() {
        return Some(deps);
    }

    // Recurse into subdirectories — but skip well-known noisy subtrees.
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        // Skip hidden dirs, target/deploy (stripped), node_modules, .git.
        if name.starts_with('.')
            || name == "node_modules"
            || name == "deploy"
            || name == "deploy-debug"
        {
            continue;
        }
        if let Some(found) = search_for_unstripped(&path, file_name, depth + 1) {
            return Some(found);
        }
    }

    None
}

/// Loads text symbols from an ELF file's symbol tables and maps them to SBPF PCs.
fn load_elf_symbols(elf_bytes: &[u8]) -> Result<BTreeMap<usize, String>> {
    let file = object::File::parse(elf_bytes)
        .map_err(|err| anyhow!("Failed to parse ELF for symbols: {err}"))?;

    let text_section = file
        .sections()
        .find(|s| s.name().ok() == Some(".text"))
        .or_else(|| {
            file.sections()
                .find(|s| s.kind() == object::SectionKind::Text)
        })
        .ok_or_else(|| anyhow!("No .text section"))?;

    let text_address = text_section.address();
    let text_end = text_address.saturating_add(text_section.size());

    let mut symbols = BTreeMap::new();

    for symbol in file.symbols().chain(file.dynamic_symbols()) {
        if symbol.kind() != SymbolKind::Text || symbol.address() == 0 {
            continue;
        }
        let Ok(name) = symbol.name() else {
            continue;
        };
        if name.is_empty() {
            continue;
        }

        let address = symbol.address();
        if address < text_address || address >= text_end {
            continue;
        }

        let relative = address - text_address;
        if relative % ebpf::INSN_SIZE as u64 != 0 {
            continue;
        }

        let pc = (relative / ebpf::INSN_SIZE as u64) as usize;
        symbols
            .entry(pc)
            .or_insert_with(|| normalize_symbol(name, pc));
    }

    Ok(symbols)
}

/// Demangles and normalizes a raw symbol for flamegraph display.
fn normalize_symbol(name: &str, pc: usize) -> String {
    let trimmed = name.trim_matches(char::from(0));
    let normalized = if trimmed.is_empty() {
        format!("function_{pc}")
    } else {
        demangle(trimmed).to_string()
    };

    let cleaned = strip_rust_hash_suffix(&normalized)
        .replace(';', ":")
        .replace('\n', " ");

    shorten_qualified_name(&cleaned)
}

/// Removes the trailing rustc symbol hash suffix when present.
fn strip_rust_hash_suffix(symbol: &str) -> &str {
    let Some((prefix, suffix)) = symbol.rsplit_once("::h") else {
        return symbol;
    };

    if suffix.len() == 16 && suffix.bytes().all(|b| b.is_ascii_hexdigit()) {
        prefix
    } else {
        symbol
    }
}

/// Shortens a fully qualified Rust name to at most the last 3 segments.
fn shorten_qualified_name(name: &str) -> String {
    // Don't shorten names that contain generic parameters or closures.
    if name.contains('<') || name.contains('{') || name.matches("::").count() <= 2 {
        return name.to_owned();
    }

    let parts: Vec<&str> = name.split("::").collect();
    if parts.len() <= 3 {
        return name.to_owned();
    }

    parts[parts.len() - 3..].join("::")
}
