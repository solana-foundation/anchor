use anyhow::{Context, Result};
use std::process::{Command, Output, Stdio};

pub struct SafeCommand {
    inner: Command,
    description: String,
}

impl SafeCommand {
    pub fn new(program: &str) -> Self {
        Self {
            inner: Command::new(program),
            description: program.to_string(),
        }
    }

    pub fn arg(mut self, arg: impl AsRef<std::ffi::OsStr>) -> Self {
        self.inner.arg(arg);
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        self.inner.args(args);
        self
    }

    pub fn env(mut self, key: &str, val: &str) -> Self {
        self.inner.env(key, val);
        self
    }

    pub fn current_dir(mut self, dir: impl AsRef<std::path::Path>) -> Self {
        self.inner.current_dir(dir);
        self
    }

    pub fn stdout(mut self, cfg: impl Into<Stdio>) -> Self {
        self.inner.stdout(cfg);
        self
    }

    pub fn stderr(mut self, cfg: impl Into<Stdio>) -> Self {
        self.inner.stderr(cfg);
        self
    }

    pub fn run_with_output(mut self) -> Result<Output> {
        self.inner
            .output()
            .with_context(|| format!("Failed to execute: {}", self.description))
    }

    pub fn run_inherit(mut self) -> Result<Output> {
        self.inner
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .with_context(|| format!("Failed to execute: {}", self.description))
    }

    pub fn spawn_with_combined_output(
        mut self,
    ) -> Result<(std::process::Child, std::io::PipeReader)> {
        let (recv, send) = std::io::pipe()?;

        let child = self
            .inner
            .stdout(send.try_clone()?)
            .stderr(send)
            .spawn()
            .with_context(|| format!("Failed to spawn: {}", self.description))?;

        Ok((child, recv))
    }
}

pub fn check_success(output: &Output, command_desc: &str) -> Result<()> {
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(crate::error::CliError::CommandFailed {
            command: command_desc.to_string(),
            reason: stderr,
        }
        .into())
    }
}
