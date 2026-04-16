//! Host-side test utilities for Anchor v2 programs.
//!
//! Drop-in replacement for `litesvm::LiteSVM::new()` that — when built
//! with the `profile` feature — records SBF register traces per test
//! under `target/anchor-v2-profile/<test_name>/`.
//!
//! `anchor test --profile` builds your tests with this feature active
//! and post-processes the trace files into flamegraphs.

pub use litesvm::LiteSVM;

#[cfg(feature = "profile")]
mod profile;

#[cfg(feature = "profile")]
pub use profile::svm;

/// When the `profile` feature is off, `svm()` is just `LiteSVM::new()`
/// with zero runtime overhead.
#[cfg(not(feature = "profile"))]
pub fn svm() -> LiteSVM {
    LiteSVM::new()
}
