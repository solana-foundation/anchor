//! Host-side test utilities for Anchor v2 programs.
//!
//! Two independent feature flags:
//!
//! - `profile` — wraps `LiteSVM` so each transaction records an SBF
//!   register trace under `target/anchor-v2-profile/<test_name>/`.
//! - `render` — consumes those trace files and produces interactive
//!   SVG flamegraphs and human-readable per-instruction listings.
//!
//! `profile` is for test code; `render` is for tooling (e.g. the
//! `anchor` CLI's `--profile` post-processing step).

#[cfg(feature = "profile")]
pub use litesvm::LiteSVM;

#[cfg(feature = "profile")]
mod profile;

#[cfg(feature = "profile")]
pub use profile::svm;

#[cfg(feature = "render")]
pub mod flamegraph;

#[cfg(feature = "render")]
pub use flamegraph::{generate_flamegraph_from_trace, print_ix_trace_to};

#[cfg(feature = "render")]
pub mod render;

#[cfg(feature = "render")]
pub use render::{render_all_tests, DEFAULT_PROFILE_DIR};
