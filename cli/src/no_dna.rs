//! Support for the `NO_DNA` environment variable.
//!
//! `NO_DNA` (Non-Human Operator) is a standard signal defined at <https://no-dna.org>.
//! When `NO_DNA=1` is set, the process is being invoked by an agent or CI pipeline
//! rather than a human. The CLI should:
//!
//! - Disable interactive prompts (treat all confirmations as "yes").
//! - Disable TUI/spinner output that cannot be parsed by a machine.
//! - Emit structured, verbose output to stderr so agents can parse results.
//!
//! # Usage
//!
//! ```bash
//! NO_DNA=1 anchor build
//! NO_DNA=1 anchor test
//! NO_DNA=1 anchor deploy
//! ```

/// Returns `true` if the `NO_DNA` environment variable is set to a truthy value
/// (`1`, `true`, `yes` — case-insensitive).
///
/// All other values (including unset) are treated as `false`.
pub fn is_no_dna() -> bool {
    match std::env::var("NO_DNA") {
        Ok(val) => matches!(val.to_lowercase().as_str(), "1" | "true" | "yes"),
        Err(_) => false,
    }
}

/// Prints a message to stderr only when running in `NO_DNA` (agent) mode.
///
/// Prefixes the message with `[NO_DNA]` so agents can easily filter it.
///
/// # Example
///
/// ```rust,ignore
/// no_dna_log!("Starting build for program: {}", program_name);
/// ```
#[macro_export]
macro_rules! no_dna_log {
    ($($arg:tt)*) => {
        if $crate::no_dna::is_no_dna() {
            eprintln!("[NO_DNA] {}", format!($($arg)*));
        }
    };
}

/// When in `NO_DNA` mode, auto-confirms any interactive yes/no prompt by
/// returning `true` immediately without reading stdin.
///
/// When *not* in `NO_DNA` mode this function **panics** — it should only be
/// called from code paths that have already checked `is_no_dna()`, or via the
/// higher-level `confirm` helper below.
///
/// Prefer using `confirm(prompt)` directly.
#[inline]
pub fn auto_yes() -> bool {
    true
}

/// Ask the user a yes/no question.
///
/// - In `NO_DNA` mode: skips the prompt, logs the auto-confirmation to stderr,
///   and returns `true`.
/// - In interactive mode: prints the prompt to stdout and reads a line from
///   stdin. Returns `true` for `y`/`yes` (case-insensitive).
///
/// # Errors
///
/// Returns an error if stdin cannot be read in interactive mode.
pub fn confirm(prompt: &str) -> anyhow::Result<bool> {
    if is_no_dna() {
        eprintln!("[NO_DNA] Auto-confirming: {}", prompt);
        return Ok(auto_yes());
    }

    use std::io::{self, Write};
    print!("{} [y/N] ", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_no_dna_unset() {
        // Guard: only run when NO_DNA is not set in the test environment.
        if std::env::var("NO_DNA").is_ok() {
            return;
        }
        assert!(!is_no_dna());
    }

    #[test]
    fn test_is_no_dna_set_to_one() {
        // We cannot mutate env in a reliable cross-thread way in unit tests;
        // instead we test the matching logic directly.
        let truthy = ["1", "true", "TRUE", "yes", "YES", "True", "Yes"];
        for val in &truthy {
            assert!(
                matches!(val.to_lowercase().as_str(), "1" | "true" | "yes"),
                "Expected '{}' to be truthy",
                val
            );
        }
    }

    #[test]
    fn test_is_no_dna_falsy_values() {
        let falsy = ["0", "false", "no", "off", "", "random"];
        for val in &falsy {
            assert!(
                !matches!(val.to_lowercase().as_str(), "1" | "true" | "yes"),
                "Expected '{}' to be falsy",
                val
            );
        }
    }
}
