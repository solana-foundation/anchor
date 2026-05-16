//! Update-channel selection for AVM.
//!
//! Lives in `~/.avm/config.toml`:
//!
//! ```toml
//! channel = "stable"  # "stable" | "pre-release" | "nightly"
//! ```
//!
//! The channel decides which artifact `avm self-update` installs and, when set
//! to `nightly`, how often the periodic update check fires (no more often than
//! every 6 hours — the cloud source for nightly artifacts is not yet wired, so
//! the install path itself is `unimplemented!`).
use {
    crate::AVM_HOME,
    anyhow::Result,
    chrono::Utc,
    clap::ValueEnum,
    serde::{Deserialize, Serialize},
    std::{fs, path::PathBuf},
};

/// Throttle for nightly update checks: at most once per 6 hours.
pub const NIGHTLY_CHECK_INTERVAL_SECS: i64 = 6 * 60 * 60;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[clap(rename_all = "kebab-case")]
pub enum Channel {
    /// Latest stable release.
    #[default]
    Stable,
    /// Latest pre-release (`alpha`/`beta`/`rc`).
    PreRelease,
    /// Bleeding-edge nightly artifact (install path not yet implemented).
    Nightly,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct AvmConfig {
    #[serde(default)]
    pub channel: Channel,
}

fn config_path() -> PathBuf {
    AVM_HOME.join("config.toml")
}

fn nightly_check_file_path() -> PathBuf {
    AVM_HOME.join(".nightly-check")
}

/// Read the persisted config, returning defaults on any error (missing file,
/// malformed TOML, etc). Configuration is non-critical state — a corrupt file
/// should never break `avm` invocations.
pub(crate) fn read_config() -> AvmConfig {
    fs::read_to_string(config_path())
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_config(cfg: &AvmConfig) -> Result<()> {
    if !AVM_HOME.exists() {
        fs::create_dir_all(&*AVM_HOME)?;
    }
    let content = toml::to_string(cfg)?;
    fs::write(config_path(), content)?;
    Ok(())
}

/// Currently selected update channel (`stable` if no config exists).
pub fn get_channel() -> Channel {
    read_config().channel
}

/// Persist `channel` to `~/.avm/config.toml`. Other config fields are preserved.
pub fn set_channel(channel: Channel) -> Result<()> {
    let mut cfg = read_config();
    cfg.channel = channel;
    write_config(&cfg)
}

/// Pure throttle decision: should a nightly check fire given the previous
/// check timestamp (if any) and current time? Extracted for testability.
pub(crate) fn should_check_nightly(last: Option<i64>, now: i64) -> bool {
    match last {
        Some(prev) => now.saturating_sub(prev) >= NIGHTLY_CHECK_INTERVAL_SECS,
        None => true,
    }
}

fn read_nightly_check_timestamp() -> Option<i64> {
    fs::read_to_string(nightly_check_file_path())
        .ok()
        .and_then(|s| s.trim().parse::<i64>().ok())
}

fn write_nightly_check_timestamp(ts: i64) {
    let _ = fs::write(nightly_check_file_path(), ts.to_string());
}

/// Run the nightly-update side of the periodic check, throttled to at most
/// once per [`NIGHTLY_CHECK_INTERVAL_SECS`].
///
/// The actual upstream poll is not yet implemented — until the cloud source
/// for nightly artifacts ships, this just records that a check happened and
/// nudges the user that the channel is active.
pub fn check_nightly_and_warn() {
    let now = Utc::now().timestamp();
    if !should_check_nightly(read_nightly_check_timestamp(), now) {
        return;
    }
    write_nightly_check_timestamp(now);
    eprintln!(
        "avm: nightly channel active — nightly update polling is not yet wired up. Once it is, \
         this check will fire at most every 6 hours."
    );
}

/// Install the latest nightly artifact. **Not implemented**: AVM does not yet
/// publish nightly artifacts to a known location, so there is nothing to
/// install. The function is shaped now so the dispatching code can route to
/// it without restructuring once the cloud source ships.
pub fn install_nightly() -> ! {
    unimplemented!(
        "nightly install path is not wired yet — cloud source for nightly artifacts pending"
    )
}

#[cfg(test)]
mod tests {
    use {super::*, crate::ensure_paths};

    // ── should_check_nightly (pure) ──────────────────────────────────────────

    #[test]
    fn nightly_check_runs_when_never_checked() {
        assert!(should_check_nightly(None, 1_000_000));
    }

    #[test]
    fn nightly_check_throttles_within_window() {
        let now = 1_000_000;
        let just_now = now - 60;
        assert!(!should_check_nightly(Some(just_now), now));
    }

    #[test]
    fn nightly_check_runs_after_window() {
        let now = 1_000_000;
        let old = now - NIGHTLY_CHECK_INTERVAL_SECS;
        assert!(should_check_nightly(Some(old), now));
        let older = now - NIGHTLY_CHECK_INTERVAL_SECS - 1;
        assert!(should_check_nightly(Some(older), now));
    }

    #[test]
    fn nightly_check_handles_future_timestamp() {
        // Clock skew safety: if the cached timestamp is somehow in the future,
        // saturating_sub yields 0 → do not check, do not crash.
        let now = 1_000_000;
        let future = now + 10_000;
        assert!(!should_check_nightly(Some(future), now));
    }

    // ── config round-trip ────────────────────────────────────────────────────

    #[test]
    fn channel_defaults_to_stable_when_no_config() {
        ensure_paths();
        // Defensive: clean up from any prior test in the same temp AVM_HOME.
        let _ = fs::remove_file(config_path());
        assert_eq!(get_channel(), Channel::Stable);
    }

    #[test]
    fn channel_round_trips_through_disk() {
        ensure_paths();
        set_channel(Channel::PreRelease).unwrap();
        assert_eq!(get_channel(), Channel::PreRelease);
        set_channel(Channel::Nightly).unwrap();
        assert_eq!(get_channel(), Channel::Nightly);
        set_channel(Channel::Stable).unwrap();
        assert_eq!(get_channel(), Channel::Stable);
    }

    #[test]
    fn malformed_config_falls_back_to_default() {
        ensure_paths();
        fs::write(config_path(), "this is not valid toml \0").unwrap();
        assert_eq!(get_channel(), Channel::Stable);
    }
}
