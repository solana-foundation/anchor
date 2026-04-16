//! Emits `ANCHOR_VERSION` env var consumed by `cli/src/lib.rs` for the
//! `anchor --version` output. Format:
//!
//!   - git checkout, clean:  `1.2.3 (abc1234)`
//!   - git checkout, dirty:  `1.2.3 (abc1234-dirty)`
//!   - not a git checkout:   `1.2.3`

use std::process::Command;

fn main() {
    // Rerun when HEAD moves or the working tree changes.
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/index");

    let manifest = env!("CARGO_MANIFEST_DIR");
    let pkg_version = env!("CARGO_PKG_VERSION");

    let hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(manifest)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let dirty = !hash.is_empty()
        && Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(manifest)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false);

    let full = if hash.is_empty() {
        pkg_version.to_string()
    } else if dirty {
        format!("{pkg_version} ({hash}-dirty)")
    } else {
        format!("{pkg_version} ({hash})")
    };

    println!("cargo:rustc-env=ANCHOR_VERSION={full}");
}
