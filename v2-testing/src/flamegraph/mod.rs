mod ix_trace;
mod svg;
mod trace;

use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Prints a human-readable per-instruction trace of a transaction to
/// `writer` by symbolicating the LiteSVM `.regs`/`.insns` files in
/// `trace_dir` against the deployed ELF at `elf_path`.
pub fn print_ix_trace_to<W: Write>(
    writer: &mut W,
    label: &str,
    elf_path: &Path,
    trace_dir: &Path,
    manifest_dir: Option<&Path>,
) -> Result<()> {
    ix_trace::print_trace_to(writer, label, elf_path, trace_dir, manifest_dir)
}

/// Generates a flamegraph SVG from LiteSVM register trace files.
pub fn generate_flamegraph_from_trace(
    program_name: &str,
    elf_path: &Path,
    trace_dir: &Path,
    output_path: &Path,
    manifest_dir: Option<&Path>,
) -> Result<()> {
    let report = trace::build_report_from_trace(program_name, elf_path, trace_dir, manifest_dir)?;
    let Some(report) = report else {
        return Ok(());
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, svg::render(&report))?;
    Ok(())
}
