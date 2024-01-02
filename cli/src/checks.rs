use std::path::Path;

use anyhow::{anyhow, Result};
use semver::Version;

use crate::{
    config::{Config, Manifest, WithPath},
    VERSION,
};

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
    in workspace root Cargo.toml",
        ))
}

/// Check whether there is a mismatch between the current CLI version and the `anchor-lang` crate
/// version.
///
/// This function logs warnings in the case of a mismatch.
pub fn check_anchor_version(cfg: &WithPath<Config>) -> Result<()> {
    let cli_version = Version::parse(VERSION)?;
    let mismatched_lang_version = cfg
        .get_rust_program_list()?
        .into_iter()
        .map(|path| path.join("Cargo.toml"))
        .map(cargo_toml::Manifest::from_path)
        .filter_map(|man| man.ok())
        .filter_map(|man| man.dependencies.get("anchor-lang").map(|d| d.to_owned()))
        .filter_map(|dep| Version::parse(dep.req()).ok())
        .find(|ver| ver != &cli_version); // Only log the warning once

    if let Some(ver) = mismatched_lang_version {
        eprintln!(
            "WARNING: `anchor-lang` version({ver}) and the current CLI version({cli_version}) \
                 don't match.\n\n\t\
                 This can lead to unwanted behavior. To use the same CLI version, add:\n\n\t\
                 [toolchain]\n\t\
                 anchor_version = \"{ver}\"\n\n\t\
                 to Anchor.toml\n"
        );
    }

    Ok(())
}
