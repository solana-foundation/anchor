use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::LintCommand;

pub(crate) const DEFAULT_LINT_REPO: &str = "https://github.com/otter-sec/anchor-lints";
const DEFAULT_PATTERN: &str = "lints/*";

/// Runs Anchor-specific lints on the workspace
pub fn run(command: LintCommand) -> Result<()> {
    // Clap autopopulates `git` with `DEFAULT_GIT_REPO`, so user may not have specified it manually
    ensure_dylint()?;
    let mut cmd = Command::new("cargo");
    cmd.arg("dylint");
    match command {
        // Default git repo provided or left blank
        LintCommand {
            git,
            path: None,
            pattern,
        } if git == DEFAULT_LINT_REPO => {
            cmd.args([
                "--git",
                git.as_ref(),
                "--pattern",
                pattern.as_deref().unwrap_or(DEFAULT_PATTERN),
            ]);
        }
        // Path provided and git is default/left blank
        LintCommand {
            git,
            path: Some(path),
            pattern: Some(pattern),
        } if git == DEFAULT_LINT_REPO => {
            cmd.args(["--path", path.as_ref(), "--pattern", &pattern]);
        }
        // Custom `--git` and `--pattern` provided
        LintCommand {
            git,
            pattern: Some(pattern),
            path: None,
        } => {
            cmd.args(["--git", git.as_ref(), "--pattern", &pattern]);
        }
        // Path provided but no pattern
        LintCommand {
            git,
            path: Some(_),
            pattern: Some(_),
        } if git == DEFAULT_LINT_REPO => {
            bail!("`--pattern` must be provided");
        }
        // Custom git but no pattern
        LintCommand { pattern: None, .. } => {
            bail!("`--pattern` must be provided");
        }
        // Path provided but `--git` also overridden
        LintCommand { path: Some(_), .. } => {
            bail!("cannot provide both `--git` and `--path`");
        }
    }

    let status = cmd.status().context("executing dylint")?;
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
        cmd.arg("cargo-dylint@5.0.0");
    }
    if !has_dylint_link {
        cmd.arg("dylint-link@5.0.0");
    }
    cmd.arg("--locked");
    if !cmd.status().context("installing dylint")?.success() {
        bail!("installing dylint failed");
    }
    Ok(())
}
