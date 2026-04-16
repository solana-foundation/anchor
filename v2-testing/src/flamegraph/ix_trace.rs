//! Human-readable per-instruction trace printer.
//!
//! Reuses the same LiteSVM `.regs` / `.insns` trace files that the flamegraph
//! generator consumes, but prints one line per retired instruction instead of
//! folding them into stacks. Each line shows:
//!
//!   - Sequence number (retirement order)
//!   - sBPF PC as `pc=NNNN`
//!   - Resolved function symbol + offset from the function's entry PC
//!   - Disassembled mnemonic + operands (from `solana_sbpf::static_analysis`)
//!   - Register diffs: only registers that changed since the previous entry
//!
//! Enable with the `BENCH_IX_TRACE=<label>` env var, e.g.
//!   `BENCH_IX_TRACE=hello_world_v2/init cargo test --test helloworld`
//! where `<label>` matches `<suite_name>/<instruction_name>`. Output goes to
//! the file `bench/ix_traces/<label>.trace` by default.

use {
    anyhow::{anyhow, Context, Result},
    solana_sbpf::{
        elf::Executable,
        program::BuiltinProgram,
        static_analysis::Analysis,
        vm::{Config, ContextObject},
    },
    std::{fs, io::Write, path::Path, sync::Arc},
};

use super::trace::{
    find_trace_files, load_function_map, lookup_function_with_pc, read_insn, read_regs,
    INSN_ENTRY_SIZE, REGS_ENTRY_SIZE,
};

/// Minimal ContextObject impl so we can parse an ELF for disassembly without
/// needing a real compute-budget tracker.
#[derive(Default)]
struct NoopContext;

impl ContextObject for NoopContext {
    fn consume(&mut self, _amount: u64) {}
    fn get_remaining(&self) -> u64 {
        0
    }
}

/// Read the `.regs` + `.insns` trace pair(s) in `trace_dir` and write a
/// human-readable per-instruction trace of the benchmarked transaction to
/// `writer`.
///
/// `elf_path` is the deployed `.so`; `manifest_dir` (optional) lets the
/// symbol loader fall back to the unstripped build artifact for Rust symbol
/// names.
pub fn print_trace_to<W: Write>(
    writer: &mut W,
    label: &str,
    elf_path: &Path,
    trace_dir: &Path,
    manifest_dir: Option<&Path>,
) -> Result<()> {
    let (symbol_map, syscall_names) = load_function_map(elf_path, manifest_dir)?;

    // Parse the executable once so we can disassemble each traced PC.
    let elf_bytes = fs::read(elf_path)
        .with_context(|| format!("Failed to read ELF file {}", elf_path.display()))?;
    let loader = Arc::new(BuiltinProgram::new_loader(Config {
        enable_symbol_and_section_labels: true,
        ..Config::default()
    }));
    let executable: Executable<NoopContext> = Executable::from_elf(&elf_bytes, loader)
        .map_err(|e| anyhow!("Failed to parse ELF for trace printing: {e}"))?;
    let analysis = Analysis::from_executable(&executable)
        .map_err(|e| anyhow!("Failed to analyze executable for trace printing: {e}"))?;

    // Build a dense PC → instruction-index map so we can look up the
    // disassembly for any traced PC in O(1). LDDW occupies two PC slots so
    // we populate both entries with the same index, matching how
    // `Analysis::disassemble_register_trace` does it upstream.
    let max_ptr = analysis
        .instructions
        .last()
        .map(|insn| insn.ptr + 2)
        .unwrap_or(0);
    let mut pc_to_idx: Vec<usize> = vec![usize::MAX; max_ptr];
    for (idx, insn) in analysis.instructions.iter().enumerate() {
        if insn.ptr < pc_to_idx.len() {
            pc_to_idx[insn.ptr] = idx;
        }
        if insn.ptr + 1 < pc_to_idx.len() {
            pc_to_idx[insn.ptr + 1] = idx;
        }
    }

    let trace_files = find_trace_files(trace_dir)?;
    if trace_files.is_empty() {
        writeln!(
            writer,
            "# {label}: no trace files in {}",
            trace_dir.display()
        )?;
        return Ok(());
    }

    writeln!(writer, "# ix trace: {label}")?;
    writeln!(
        writer,
        "# elf: {}  ({} unique instructions in analysis)",
        elf_path.display(),
        analysis.instructions.len()
    )?;
    writeln!(writer, "#")?;
    writeln!(
        writer,
        "#  seq  pc       function+offset                                                     mnemonic                                 reg diffs"
    )?;

    let mut seq: usize = 0;
    let mut prev_regs: Option<[u64; 12]> = None;
    let mut per_function: std::collections::BTreeMap<String, u64> = Default::default();

    for (regs_path, insns_path) in &trace_files {
        let regs_data = fs::read(regs_path)
            .with_context(|| format!("Failed to read {}", regs_path.display()))?;
        let insns_data = fs::read(insns_path)
            .with_context(|| format!("Failed to read {}", insns_path.display()))?;

        let entry_count = regs_data.len() / REGS_ENTRY_SIZE;
        let insn_count = insns_data.len() / INSN_ENTRY_SIZE;
        let count = entry_count.min(insn_count);

        for i in 0..count {
            let regs = read_regs(&regs_data, i);
            let insn_bytes = read_insn(&insns_data, i);
            let pc = regs[11];

            // Symbol + offset
            let (fn_name, entry_pc) = lookup_function_with_pc(&symbol_map, pc);
            let offset = pc.saturating_sub(entry_pc);
            *per_function.entry(fn_name.clone()).or_default() += 1;

            // Disassembly via solana_sbpf's analysis
            let mnemonic = {
                let pc_usize = pc as usize;
                if pc_usize < pc_to_idx.len() && pc_to_idx[pc_usize] != usize::MAX {
                    let idx = pc_to_idx[pc_usize];
                    let insn = &analysis.instructions[idx];
                    analysis.disassemble_instruction(insn, pc_usize)
                } else {
                    // PC not in analysis coverage — maybe a syscall stub, maybe
                    // a hash-only reference. Fall back to raw opcode byte.
                    format!("<pc {pc:#x} oob>  raw={insn_bytes:02x?}")
                }
            };

            // Annotate syscalls with their resolved name if we can figure it
            // out from the CALL_IMM immediate.
            let syscall_suffix = if insn_bytes[0] == solana_sbpf::ebpf::CALL_IMM {
                let imm = u32::from_le_bytes(insn_bytes[4..8].try_into().unwrap());
                // Syscall = PC stays put for 1 step (runtime returns to pc+1).
                // We can't always tell from a single line; just annotate
                // whenever the imm matches a known syscall hash.
                syscall_names
                    .get(&imm)
                    .map(|name| format!("  → syscall:{name}"))
                    .unwrap_or_default()
            } else {
                String::new()
            };

            // Register diffs (R0..R10; R11 = PC is implied by the pc column).
            let diffs = match prev_regs {
                None => {
                    // First line: show all non-zero regs.
                    let mut out = String::new();
                    for r in 0..11 {
                        if regs[r] != 0 {
                            if !out.is_empty() {
                                out.push(' ');
                            }
                            out.push_str(&format!("r{r}={:#x}", regs[r]));
                        }
                    }
                    if out.is_empty() {
                        "(all zero)".to_string()
                    } else {
                        out
                    }
                }
                Some(prev) => {
                    let mut out = String::new();
                    for r in 0..11 {
                        if regs[r] != prev[r] {
                            if !out.is_empty() {
                                out.push(' ');
                            }
                            out.push_str(&format!("r{r}={:#x}", regs[r]));
                        }
                    }
                    out
                }
            };

            writeln!(
                writer,
                "{seq:>5}  {pc:>6}  {fn_name}+{offset:<#6x}  {mnemonic:<40}  {diffs}{syscall_suffix}"
            )?;

            seq += 1;
            prev_regs = Some(regs);
        }
    }

    writeln!(writer, "#")?;
    writeln!(writer, "# total retired instructions: {seq}")?;
    writeln!(writer, "#")?;
    writeln!(writer, "# per-function instruction count:")?;
    let mut sorted: Vec<_> = per_function.iter().collect();
    sorted.sort_by_key(|(_, v)| std::cmp::Reverse(**v));
    for (name, n) in sorted {
        writeln!(writer, "#   {n:>5}  {name}")?;
    }

    Ok(())
}
