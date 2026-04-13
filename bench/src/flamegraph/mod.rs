mod svg;
mod trace;
pub mod walker;

use anyhow::Result;
use std::fs;
use std::path::Path;

/// Generates a flamegraph SVG from LiteSVM register trace files.
///
/// `trace_dir` should contain the `.regs` and `.insns` files produced by
/// running a transaction with `LiteSVM::new_debuggable(true)`.
///
/// `manifest_dir` is an optional pointer to the program's Cargo manifest
/// directory — when supplied, symbol lookup prefers the unstripped binary
/// inside that workspace's target tree (avoiding ambiguity when the same
/// lib name appears in multiple workspaces).
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

/// Old static analysis approach (kept for reference but no longer called by default).
#[allow(dead_code)]
pub fn generate_flamegraph(
    program_name: &str,
    elf_path: &Path,
    output_path: &Path,
) -> Result<()> {
    let report = walker::analyze_program(program_name, elf_path)?;
    let Some(report) = report else {
        return Ok(());
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, svg::render(&report))?;
    Ok(())
}
