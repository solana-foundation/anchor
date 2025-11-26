//! When building for linting, enable the `dylint` cfg to enable some utilities

use std::env;

fn main() {
    println!("cargo::rerun-if-env-changed=DYLINT_LIBS");
    if env::var("DYLINT_LIBS").is_ok() {
        println!("cargo::rustc-cfg=dylint");
    }
}
