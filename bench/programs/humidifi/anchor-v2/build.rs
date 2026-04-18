//! Stage `humidifi.so` into this crate's `target/deploy/` under the
//! program-id filename, which is what `anchor debugger`'s loose-mode
//! discovery walks (cli/src/debugger/loose.rs). Keeps the committed
//! binary in the crate dir (source of truth) while still satisfying
//! the `target/deploy/<pubkey>.so` layout the debugger expects.

use std::path::PathBuf;

const PROGRAM_ID: &str = "9H6tua7jkLhdm3w8BvgpTn5LZNU7g4ZynDmCiNN3q6Rp";

fn main() {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest_dir.join("humidifi.so");

    // This crate is its own nested workspace, so `target/` lives right
    // inside the manifest dir.
    let deploy = manifest_dir.join("target").join("deploy");
    std::fs::create_dir_all(&deploy).expect("create target/deploy/");

    let dst = deploy.join(format!("{PROGRAM_ID}.so"));
    std::fs::copy(&src, &dst).unwrap_or_else(|e| {
        panic!("copy {} -> {}: {e}", src.display(), dst.display())
    });

    println!("cargo:rerun-if-changed={}", src.display());
}
