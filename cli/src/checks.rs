use std::path::Path;

use anyhow::{anyhow, Result};

use crate::config::Manifest;

/// Check whether `overflow-checks` codegen option is enabled.
///
/// https://doc.rust-lang.org/rustc/codegen-options/index.html#overflow-checks
pub fn check_overflow(cargo_toml_path: impl AsRef<Path>) -> Result<bool> {
    Manifest::from_path(cargo_toml_path)?
        .profile
        .release
        .as_ref()
        .and_then(|profile| profile.overflow_checks)
        .ok_or(anyhow!(
            "`overflow-checks` is not enabled. To enable, add:\n\n\
    [profile.release]\n\
    overflow-checks = true\n\n\
    in workspace root Cargo.toml.",
        ))
}
