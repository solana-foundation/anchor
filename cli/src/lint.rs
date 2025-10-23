use std::process::Command;

use anyhow::{bail, Context, Result};

/// Runs Anchor-specific lints on the workspace
pub fn run(path: &str, pattern: &str) -> Result<()> {
    ensure_dylint()?;
    let status = Command::new("cargo")
        .args(["dylint", "--path", path, "--pattern", pattern])
        .status()
        .context("executing dylint")?;
    if !status.success() {
        bail!("dylint did not execute successfully");
    }
    Ok(())
}

/// Ensures dylint is installed
fn ensure_dylint() -> Result<()> {
    let output = Command::new("cargo")
        .arg("install")
        .arg("--list")
        .output()?;
    let mut has_cargo_dylint = false;
    let mut has_dylint_link = false;
    for line in String::from_utf8(output.stdout)
        .context("parsing `cargo install --list` output")?
        .lines()
    {
        let line = line.trim();
        if line == "cargo-dylint" {
            has_cargo_dylint = true;
        } else if line == "dylint-link" {
            has_dylint_link = true;
        }
        if has_cargo_dylint && has_dylint_link {
            break;
        }
    }
    if has_cargo_dylint && has_dylint_link {
        return Ok(());
    }

    eprintln!("Installing required packages");
    let mut cmd = Command::new("cargo");
    cmd.arg("install");
    if !has_cargo_dylint {
        cmd.arg("cargo-dylint");
    }
    if !has_dylint_link {
        cmd.arg("dylint-link");
    }

    if !cmd.status().context("installing dylint")?.success() {
        bail!("installing dylint failed");
    }
    Ok(())
}
