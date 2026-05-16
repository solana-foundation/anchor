//! Platform-tools version resolution.
//!
//! Given a Solana version a project targets, return the `platform-tools` version
//! that ships with that Solana release. The map is embedded at compile time from
//! `../platform-tools-map.toml` and ordered by ascending Solana version, so
//! resolution is a linear floor lookup: pick the entry with the largest
//! `solana` key that is `<= requested`.
//!
//! When the project pins no Solana version at all, fall back to the map's
//! `fallback` field (kept equal to the newest entry's `platform_tools`).
use {
    crate::resolve::{resolve_solana_version, SolanaResolution, SolanaResolutionSource},
    anyhow::{anyhow, Context, Result},
    semver::Version,
    serde::Deserialize,
    std::{path::Path, sync::LazyLock},
};

const PLATFORM_TOOLS_MAP_TOML: &str = include_str!("../platform-tools-map.toml");

#[derive(Debug, Deserialize)]
struct PlatformToolsMap {
    fallback: String,
    entries: Vec<MapEntry>,
}

#[derive(Debug, Deserialize)]
struct MapEntry {
    solana: String,
    platform_tools: String,
}

/// Parsed and validated form of the static map.
#[derive(Debug)]
struct ParsedMap {
    fallback: String,
    /// Sorted ascending by Solana version.
    entries: Vec<(Version, String)>,
}

static MAP: LazyLock<ParsedMap> = LazyLock::new(|| {
    let raw: PlatformToolsMap = toml::from_str(PLATFORM_TOOLS_MAP_TOML)
        .expect("Built-in platform-tools-map.toml must parse");

    let mut entries: Vec<(Version, String)> = raw
        .entries
        .into_iter()
        .map(|e| {
            let v = Version::parse(&e.solana).unwrap_or_else(|err| {
                panic!("Invalid Solana version `{}` in map: {err}", e.solana)
            });
            (v, e.platform_tools)
        })
        .collect();

    let was_sorted = entries.windows(2).all(|w| w[0].0 <= w[1].0);
    assert!(
        was_sorted,
        "platform-tools-map.toml entries must be sorted by Solana version"
    );
    let _ = was_sorted; // silence unused-variable in release if the assert is stripped
    entries.sort_by(|a, b| a.0.cmp(&b.0)); // defensive

    ParsedMap {
        fallback: raw.fallback,
        entries,
    }
});

/// Where the platform-tools version came from. Combines the upstream Solana
/// source (if any) with the lookup outcome (a specific map row, or the
/// fallback because nothing matched).
#[derive(Debug, Clone)]
pub enum PlatformToolsSource {
    /// Mapped from a project-pinned Solana version via the static map.
    Mapped {
        solana: Version,
        solana_source: SolanaResolutionSource,
    },
    /// Project pinned a Solana version older than the map's earliest entry.
    /// We still return the oldest known platform-tools.
    BelowMap {
        solana: Version,
        solana_source: SolanaResolutionSource,
    },
    /// Project did not pin Solana → use the map's hardcoded fallback.
    Fallback,
}

impl PlatformToolsSource {
    pub fn describe(&self) -> String {
        match self {
            Self::Mapped {
                solana,
                solana_source,
            } => format!("solana {solana} → map ({})", solana_source.describe()),
            Self::BelowMap {
                solana,
                solana_source,
            } => format!(
                "solana {solana} predates map; using earliest entry ({})",
                solana_source.describe()
            ),
            Self::Fallback => "fallback (no Solana version pinned)".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlatformToolsResolution {
    /// e.g. `"v1.54"`. Kept as a string because upstream uses the `v`-prefixed
    /// form everywhere (release tags, archive names, the `DEFAULT_…` constant).
    pub version: String,
    pub source: PlatformToolsSource,
}

/// Resolve the platform-tools version for the project rooted at `start`.
///
/// Walks the same project-detection logic as [`resolve_solana_version`], then
/// performs a floor lookup in the embedded map.
pub fn resolve_platform_tools(start: &Path) -> Result<PlatformToolsResolution> {
    match resolve_solana_version(start)? {
        Some(solana_res) => Ok(resolve_for_solana(&solana_res)),
        None => Ok(PlatformToolsResolution {
            version: MAP.fallback.clone(),
            source: PlatformToolsSource::Fallback,
        }),
    }
}

fn resolve_for_solana(solana_res: &SolanaResolution) -> PlatformToolsResolution {
    let solana = &solana_res.version;
    let entries = &MAP.entries;

    // Floor lookup: largest entry.0 <= solana.
    let pick = entries.iter().rposition(|(v, _)| v <= solana);
    match pick {
        Some(idx) => PlatformToolsResolution {
            version: entries[idx].1.clone(),
            source: PlatformToolsSource::Mapped {
                solana: solana.clone(),
                solana_source: solana_res.source.clone(),
            },
        },
        None => {
            // Requested Solana is older than every entry. Return the earliest
            // known platform-tools rather than the (newer) fallback — older
            // toolchains are closer to what such a project expects.
            let earliest = &entries
                .first()
                .expect("platform-tools map must have at least one entry")
                .1;
            PlatformToolsResolution {
                version: earliest.clone(),
                source: PlatformToolsSource::BelowMap {
                    solana: solana.clone(),
                    solana_source: solana_res.source.clone(),
                },
            }
        }
    }
}

/// Look up the platform-tools version for an explicit Solana version without
/// touching the filesystem. Useful for callers that already have a Solana
/// version in hand.
pub fn lookup_for_solana_version(solana: &Version) -> Result<String> {
    let entries = &MAP.entries;
    entries
        .iter()
        .rposition(|(v, _)| v <= solana)
        .map(|idx| entries[idx].1.clone())
        .ok_or_else(|| {
            anyhow!(
                "Solana {solana} predates the earliest platform-tools map entry ({}).",
                entries[0].0
            )
        })
}

/// The hardcoded fallback platform-tools version. Exposed for callers that
/// want to surface it to the user (e.g. `avm platform-tools resolve`).
pub fn fallback_version() -> &'static str {
    &MAP.fallback
}

/// Force the map to parse at startup, surfacing any embedded-data bugs as a
/// clear error instead of a panic in a random first user.
pub fn validate_embedded_map() -> Result<()> {
    let raw: PlatformToolsMap = toml::from_str(PLATFORM_TOOLS_MAP_TOML)
        .context("Parsing embedded platform-tools-map.toml")?;
    for e in &raw.entries {
        Version::parse(&e.solana)
            .with_context(|| format!("Invalid Solana version `{}` in map", e.solana))?;
    }
    if raw.entries.is_empty() {
        return Err(anyhow!(
            "platform-tools-map.toml must have at least one entry"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use {super::*, crate::resolve::SolanaResolutionSource, std::path::PathBuf};

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn fake_solana(s: &str) -> SolanaResolution {
        SolanaResolution {
            version: v(s),
            source: SolanaResolutionSource::AnchorToml(PathBuf::from("Anchor.toml")),
        }
    }

    // ── Embedded map ─────────────────────────────────────────────────────────

    #[test]
    fn embedded_map_parses_and_is_sorted() {
        validate_embedded_map().unwrap();
        let entries = &MAP.entries;
        assert!(entries.len() >= 2);
        assert!(entries.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn fallback_matches_newest_entry() {
        // Sanity: the fallback should equal the newest entry's platform_tools,
        // per the comment in platform-tools-map.toml.
        let newest = &MAP.entries.last().unwrap().1;
        assert_eq!(&MAP.fallback, newest);
    }

    // ── Floor lookup ────────────────────────────────────────────────────────

    #[test]
    fn exact_entry_match() {
        let res = resolve_for_solana(&fake_solana("3.0.0"));
        assert_eq!(res.version, "v1.51");
        assert!(matches!(res.source, PlatformToolsSource::Mapped { .. }));
    }

    #[test]
    fn between_entries_picks_floor() {
        // 2.2.5 sits between (2.2.3 → v1.45) and (2.2.8 → v1.46) → floor is v1.45.
        let res = resolve_for_solana(&fake_solana("2.2.5"));
        assert_eq!(res.version, "v1.45");
    }

    #[test]
    fn above_all_entries_uses_latest() {
        let res = resolve_for_solana(&fake_solana("99.0.0"));
        let latest = &MAP.entries.last().unwrap().1;
        assert_eq!(&res.version, latest);
    }

    #[test]
    fn below_all_entries_uses_earliest() {
        let res = resolve_for_solana(&fake_solana("1.0.0"));
        let earliest = &MAP.entries.first().unwrap().1;
        assert_eq!(&res.version, earliest);
        assert!(matches!(res.source, PlatformToolsSource::BelowMap { .. }));
    }

    #[test]
    fn lookup_for_solana_version_works() {
        assert_eq!(lookup_for_solana_version(&v("3.0.0")).unwrap(), "v1.51");
        assert_eq!(lookup_for_solana_version(&v("4.5.0")).unwrap(), "v1.54");
        // Below earliest → error from this lower-level helper.
        assert!(lookup_for_solana_version(&v("0.1.0")).is_err());
    }

    // ── Specific known transitions ──────────────────────────────────────────

    #[test]
    fn known_transition_1_18_0_to_v1_39() {
        assert_eq!(lookup_for_solana_version(&v("1.18.0")).unwrap(), "v1.39");
    }

    #[test]
    fn known_transition_1_18_8_to_v1_41() {
        assert_eq!(lookup_for_solana_version(&v("1.18.8")).unwrap(), "v1.41");
    }

    #[test]
    fn known_transition_2_0_5_to_v1_42() {
        assert_eq!(lookup_for_solana_version(&v("2.0.5")).unwrap(), "v1.42");
    }

    #[test]
    fn known_transition_2_1_0_to_v1_43() {
        assert_eq!(lookup_for_solana_version(&v("2.1.0")).unwrap(), "v1.43");
    }

    #[test]
    fn known_transition_3_0_0_to_v1_51() {
        assert_eq!(lookup_for_solana_version(&v("3.0.0")).unwrap(), "v1.51");
    }

    #[test]
    fn known_transition_4_0_0_to_v1_54() {
        assert_eq!(lookup_for_solana_version(&v("4.0.0")).unwrap(), "v1.54");
    }
}
