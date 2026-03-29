//! NO_DNA (Non-Human Operator) support.
//!
//! When the `NO_DNA` environment variable is set to `1`, the CLI
//! operates in a non-interactive, machine-friendly mode:
//!
//! - Interactive prompts and TUI elements are disabled
//! - Output is structured and verbose
//! - Suitable for agents, CI pipelines, and other automated contexts
//!
//! Usage:
//! ```bash
//! NO_DNA=1 anchor build
//! NO_DNA=1 anchor test
//! NO_DNA=1 anchor deploy
//! ```
//!
//! See <https://no-dna.org> for the full standard.

/// Returns `true` when the `NO_DNA` env var is set to `"1"`.
///
/// This signals that the caller is a non-human operator (agent, CI, etc.)
/// and the CLI should suppress interactive prompts and TUI elements.
pub fn is_no_dna() -> bool {
    std::env::var("NO_DNA").map(|v| v == "1").unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_dna_unset_returns_false() {
        // Ensure NO_DNA is not set for this test
        std::env::remove_var("NO_DNA");
        assert!(!is_no_dna());
    }

    #[test]
    fn no_dna_set_to_one_returns_true() {
        std::env::set_var("NO_DNA", "1");
        assert!(is_no_dna());
        std::env::remove_var("NO_DNA");
    }

    #[test]
    fn no_dna_set_to_other_returns_false() {
        std::env::set_var("NO_DNA", "true");
        assert!(!is_no_dna());
        std::env::remove_var("NO_DNA");

        std::env::set_var("NO_DNA", "0");
        assert!(!is_no_dna());
        std::env::remove_var("NO_DNA");
    }
}
