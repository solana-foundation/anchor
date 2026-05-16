//! Project-aware Anchor version resolution for AVM.
//!
//! Resolves the Anchor version to use for a given working directory by
//! walking up the filesystem and consulting, in priority order:
//!
//! 1. `[toolchain] anchor_version` in `Anchor.toml` (the canonical source).
//! 2. The legacy `.anchorversion` file (preserved for back-compat).
//! 3. The `anchor-lang` dependency declared in the workspace's program
//!    `Cargo.toml` (with workspace inheritance support).
//! 4. The global default recorded in `~/.avm/.version` (set by `avm use`).
//!
//! This module is intentionally side-effect-free: it never installs, prompts,
//! or writes to disk. Callers decide what to do with the resolution.
use {
    crate::{current_version, read_installed_versions},
    anyhow::{anyhow, bail, Context, Result},
    cargo_toml::{Dependency, Manifest},
    semver::{Version, VersionReq},
    serde::Deserialize,
    std::{
        fs,
        path::{Path, PathBuf},
    },
};

/// Where a resolved Anchor version came from. Useful for diagnostics and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionSource {
    /// `[toolchain] anchor_version` in the given `Anchor.toml`.
    AnchorToml(PathBuf),
    /// Legacy `.anchorversion` file at the given path.
    AnchorVersionFile(PathBuf),
    /// `anchor-lang` dependency in the given `Cargo.toml`.
    CargoToml(PathBuf),
    /// `~/.avm/.version` (set by `avm use`).
    GlobalDefault,
}

impl ResolutionSource {
    pub fn describe(&self) -> String {
        match self {
            Self::AnchorToml(p) => format!("[toolchain] anchor_version in {}", p.display()),
            Self::AnchorVersionFile(p) => format!(".anchorversion at {}", p.display()),
            Self::CargoToml(p) => format!("anchor-lang dep in {}", p.display()),
            Self::GlobalDefault => "global default (avm use)".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Resolution {
    pub version: Version,
    pub source: ResolutionSource,
}

/// Resolve the Anchor version for `start` against the live `read_installed_versions()`.
///
/// Returns `Ok(None)` only if every source failed (no project context AND no
/// global default set). Errors propagate from malformed manifests or
/// unsatisfiable `VersionReq` predicates so the caller can surface them.
pub fn resolve_anchor_version(start: &Path) -> Result<Option<Resolution>> {
    let installed = read_installed_versions().unwrap_or_default();
    resolve_anchor_version_with(start, &installed, current_version().ok())
}

/// Variant that takes installed versions and the global default explicitly,
/// for testing without touching `$AVM_HOME`.
pub fn resolve_anchor_version_with(
    start: &Path,
    installed: &[Version],
    global_default: Option<Version>,
) -> Result<Option<Resolution>> {
    if let Some(res) = resolve_from_anchor_toml(start)? {
        return Ok(Some(res));
    }
    if let Some(res) = resolve_from_anchorversion_file(start)? {
        return Ok(Some(res));
    }
    if let Some(res) = resolve_from_cargo_toml(start, installed)? {
        return Ok(Some(res));
    }
    Ok(global_default.map(|version| Resolution {
        version,
        source: ResolutionSource::GlobalDefault,
    }))
}

// ── Anchor.toml ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AnchorToml {
    toolchain: Option<AnchorToolchain>,
}

#[derive(Deserialize)]
struct AnchorToolchain {
    #[serde(default)]
    anchor_version: Option<String>,
}

fn resolve_from_anchor_toml(start: &Path) -> Result<Option<Resolution>> {
    let Some(path) = find_ancestor_file(start, "Anchor.toml") else {
        return Ok(None);
    };
    let text = fs::read_to_string(&path).with_context(|| format!("Reading {}", path.display()))?;
    let parsed: AnchorToml =
        toml::from_str(&text).with_context(|| format!("Parsing {}", path.display()))?;
    let Some(ver_str) = parsed.toolchain.and_then(|t| t.anchor_version) else {
        return Ok(None);
    };
    let version = Version::parse(&ver_str).with_context(|| {
        format!(
            "Parsing [toolchain] anchor_version = \"{ver_str}\" in {}",
            path.display()
        )
    })?;
    Ok(Some(Resolution {
        version,
        source: ResolutionSource::AnchorToml(path),
    }))
}

// ── .anchorversion ───────────────────────────────────────────────────────────

fn resolve_from_anchorversion_file(start: &Path) -> Result<Option<Resolution>> {
    let Some(path) = find_ancestor_file(start, ".anchorversion") else {
        return Ok(None);
    };
    let text = fs::read_to_string(&path).with_context(|| format!("Reading {}", path.display()))?;
    let version = Version::parse(text.trim())
        .with_context(|| format!("Parsing version in {}", path.display()))?;
    Ok(Some(Resolution {
        version,
        source: ResolutionSource::AnchorVersionFile(path),
    }))
}

// ── Cargo.toml ───────────────────────────────────────────────────────────────

fn resolve_from_cargo_toml(start: &Path, installed: &[Version]) -> Result<Option<Resolution>> {
    let manifests = candidate_program_manifests(start);

    for manifest_path in manifests {
        let manifest = match Manifest::from_path(&manifest_path) {
            Ok(m) => m,
            // `from_path` can fail on incomplete workspace data; treat as
            // "no anchor-lang here" rather than aborting the whole resolution.
            Err(_) => continue,
        };
        let Some(dep) = manifest.dependencies.get("anchor-lang") else {
            continue;
        };
        if let Some(version) = anchor_lang_version(dep, &manifest_path, installed)? {
            return Ok(Some(Resolution {
                version,
                source: ResolutionSource::CargoToml(manifest_path),
            }));
        }
    }

    Ok(None)
}

/// Build the ordered list of `Cargo.toml` paths to consult.
///
/// Prefers the Anchor workspace's `programs/*/Cargo.toml` when an `Anchor.toml`
/// is found; otherwise falls back to the nearest `Cargo.toml` walking upward.
fn candidate_program_manifests(start: &Path) -> Vec<PathBuf> {
    if let Some(anchor_toml) = find_ancestor_file(start, "Anchor.toml") {
        let workspace_root = anchor_toml.parent().unwrap_or(Path::new("."));
        let mut out: Vec<PathBuf> = Vec::new();
        let programs_dir = workspace_root.join("programs");
        if let Ok(entries) = fs::read_dir(&programs_dir) {
            for entry in entries.flatten() {
                let candidate = entry.path().join("Cargo.toml");
                if candidate.is_file() {
                    out.push(candidate);
                }
            }
        }
        // Deterministic order so tests don't depend on filesystem iteration.
        out.sort();
        // Workspace root as last-resort candidate.
        let root_cargo = workspace_root.join("Cargo.toml");
        if root_cargo.is_file() {
            out.push(root_cargo);
        }
        return out;
    }
    // No Anchor.toml — try the nearest ancestor Cargo.toml.
    find_ancestor_file(start, "Cargo.toml")
        .into_iter()
        .collect()
}

/// Extract a concrete Anchor version from an `anchor-lang` dependency entry,
/// rejecting git/path deps and resolving workspace inheritance manually if
/// `cargo_toml`'s auto-resolution didn't fill it in.
fn anchor_lang_version(
    dep: &Dependency,
    manifest_path: &Path,
    installed: &[Version],
) -> Result<Option<Version>> {
    match dep {
        Dependency::Simple(req) => Ok(Some(resolve_version_req(req, installed)?)),
        Dependency::Detailed(detail) => {
            if detail.git.is_some() {
                bail!(
                    "`anchor-lang` in {} is a git dependency, which AVM cannot map to a released \
                     version. Pin `[toolchain] anchor_version` in `Anchor.toml` to override.",
                    manifest_path.display()
                );
            }
            if detail.path.is_some() {
                bail!(
                    "`anchor-lang` in {} is a path dependency. Pin `[toolchain] anchor_version` \
                     in `Anchor.toml` to override.",
                    manifest_path.display()
                );
            }
            match &detail.version {
                Some(req) => Ok(Some(resolve_version_req(req, installed)?)),
                None => Ok(None),
            }
        }
        Dependency::Inherited(_) => {
            // `Manifest::from_path` normally completes workspace inheritance,
            // but if it didn't (e.g. workspace root missing during a test),
            // climb to find the workspace root ourselves.
            resolve_workspace_anchor_lang(manifest_path, installed)
        }
    }
}

/// Walk up from a member crate's `Cargo.toml` looking for a workspace root
/// that declares `[workspace.dependencies] anchor-lang`.
fn resolve_workspace_anchor_lang(
    member_manifest: &Path,
    installed: &[Version],
) -> Result<Option<Version>> {
    let mut cur = member_manifest.parent().and_then(Path::parent);
    while let Some(dir) = cur {
        let candidate = dir.join("Cargo.toml");
        if candidate.is_file() {
            if let Ok(manifest) = Manifest::from_path(&candidate) {
                if let Some(ws) = manifest.workspace.as_ref() {
                    if let Some(dep) = ws.dependencies.get("anchor-lang") {
                        return anchor_lang_version(dep, &candidate, installed);
                    }
                }
            }
        }
        cur = dir.parent();
    }
    Ok(None)
}

/// Resolve a version-requirement string against a set of installed versions.
///
/// An exact semver (`1.2.3`, `1.0.0-rc.3`) short-circuits and is returned as-is,
/// even if not installed — the caller decides whether to install it. A range
/// (`^0.30`, `>=0.29, <0.31`) requires at least one matching installed version.
fn resolve_version_req(req_str: &str, installed: &[Version]) -> Result<Version> {
    if let Ok(v) = Version::parse(req_str) {
        return Ok(v);
    }
    let req = VersionReq::parse(req_str)
        .with_context(|| format!("Parsing version requirement `{req_str}`"))?;
    installed
        .iter()
        .filter(|v| req.matches(v))
        .max()
        .cloned()
        .ok_or_else(|| {
            anyhow!(
                "No installed Anchor version satisfies `{req_str}`. Run `avm install` for a \
                 matching version."
            )
        })
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn find_ancestor_file(start: &Path, name: &str) -> Option<PathBuf> {
    let mut cur: Option<&Path> = Some(start);
    while let Some(dir) = cur {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        cur = dir.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use {super::*, std::fs, tempfile::TempDir};

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn write(p: &Path, contents: &str) {
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, contents).unwrap();
    }

    #[test]
    fn anchor_toml_snake_case_wins() {
        let dir = TempDir::new().unwrap();
        write(
            &dir.path().join("Anchor.toml"),
            "[toolchain]\nanchor_version = \"0.30.1\"\n",
        );
        let res = resolve_anchor_version_with(dir.path(), &[], None)
            .unwrap()
            .unwrap();
        assert_eq!(res.version, v("0.30.1"));
        assert!(matches!(res.source, ResolutionSource::AnchorToml(_)));
    }

    #[test]
    fn anchor_toml_walks_up_from_subdir() {
        let dir = TempDir::new().unwrap();
        write(
            &dir.path().join("Anchor.toml"),
            "[toolchain]\nanchor_version = \"0.29.0\"\n",
        );
        let nested = dir.path().join("programs").join("my-program").join("src");
        fs::create_dir_all(&nested).unwrap();
        let res = resolve_anchor_version_with(&nested, &[], None)
            .unwrap()
            .unwrap();
        assert_eq!(res.version, v("0.29.0"));
    }

    #[test]
    fn anchor_toml_missing_toolchain_falls_through() {
        let dir = TempDir::new().unwrap();
        write(
            &dir.path().join("Anchor.toml"),
            "[provider]\ncluster = \"localnet\"\n",
        );
        write(&dir.path().join(".anchorversion"), "0.28.0\n");
        let res = resolve_anchor_version_with(dir.path(), &[], None)
            .unwrap()
            .unwrap();
        assert_eq!(res.version, v("0.28.0"));
        assert!(matches!(res.source, ResolutionSource::AnchorVersionFile(_)));
    }

    #[test]
    fn anchorversion_file_used_when_no_anchor_toml() {
        let dir = TempDir::new().unwrap();
        write(&dir.path().join(".anchorversion"), "0.27.0");
        let res = resolve_anchor_version_with(dir.path(), &[], None)
            .unwrap()
            .unwrap();
        assert_eq!(res.version, v("0.27.0"));
    }

    #[test]
    fn cargo_toml_simple_anchor_lang_dep() {
        let dir = TempDir::new().unwrap();
        write(
            &dir.path().join("Anchor.toml"),
            "[provider]\ncluster = \"localnet\"\n",
        );
        write(
            &dir.path().join("programs/foo/Cargo.toml"),
            "[package]\nname = \"foo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[lib]\npath = \
             \"src/lib.rs\"\n[dependencies]\nanchor-lang = \"0.30.1\"\n",
        );
        // src/lib.rs must exist for cargo_toml's completion step.
        write(&dir.path().join("programs/foo/src/lib.rs"), "");
        let res = resolve_anchor_version_with(dir.path(), &[], None)
            .unwrap()
            .unwrap();
        assert_eq!(res.version, v("0.30.1"));
        assert!(matches!(res.source, ResolutionSource::CargoToml(_)));
    }

    #[test]
    fn cargo_toml_workspace_inheritance() {
        let dir = TempDir::new().unwrap();
        write(&dir.path().join("Anchor.toml"), "");
        write(
            &dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"programs/foo\"]\nresolver = \
             \"2\"\n[workspace.dependencies]\nanchor-lang = \"0.30.0\"\n",
        );
        write(
            &dir.path().join("programs/foo/Cargo.toml"),
            "[package]\nname = \"foo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[lib]\npath = \
             \"src/lib.rs\"\n[dependencies]\nanchor-lang = { workspace = true }\n",
        );
        write(&dir.path().join("programs/foo/src/lib.rs"), "");
        let res = resolve_anchor_version_with(dir.path(), &[], None)
            .unwrap()
            .unwrap();
        assert_eq!(res.version, v("0.30.0"));
    }

    #[test]
    fn cargo_toml_version_req_selects_highest_installed() {
        let dir = TempDir::new().unwrap();
        write(&dir.path().join("Anchor.toml"), "");
        write(
            &dir.path().join("programs/foo/Cargo.toml"),
            "[package]\nname = \"foo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[lib]\npath = \
             \"src/lib.rs\"\n[dependencies]\nanchor-lang = \"^0.29\"\n",
        );
        write(&dir.path().join("programs/foo/src/lib.rs"), "");
        let installed = [v("0.28.0"), v("0.29.0"), v("0.29.3"), v("0.30.0")];
        let res = resolve_anchor_version_with(dir.path(), &installed, None)
            .unwrap()
            .unwrap();
        assert_eq!(res.version, v("0.29.3"));
    }

    #[test]
    fn cargo_toml_git_dep_errors() {
        let dir = TempDir::new().unwrap();
        write(&dir.path().join("Anchor.toml"), "");
        write(
            &dir.path().join("programs/foo/Cargo.toml"),
            "[package]\nname = \"foo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
             [lib]\npath = \"src/lib.rs\"\n\
             [dependencies]\nanchor-lang = { git = \"https://github.com/solana-foundation/anchor\" }\n",
        );
        write(&dir.path().join("programs/foo/src/lib.rs"), "");
        let err = resolve_anchor_version_with(dir.path(), &[], None).unwrap_err();
        assert!(err.to_string().contains("git dependency"));
    }

    #[test]
    fn cargo_toml_path_dep_errors() {
        let dir = TempDir::new().unwrap();
        write(&dir.path().join("Anchor.toml"), "");
        write(&dir.path().join("local/anchor-lang/Cargo.toml"), "");
        write(
            &dir.path().join("programs/foo/Cargo.toml"),
            "[package]\nname = \"foo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[lib]\npath = \
             \"src/lib.rs\"\n[dependencies]\nanchor-lang = { path = \"../../local/anchor-lang\" \
             }\n",
        );
        write(&dir.path().join("programs/foo/src/lib.rs"), "");
        let err = resolve_anchor_version_with(dir.path(), &[], None).unwrap_err();
        assert!(err.to_string().contains("path dependency"));
    }

    #[test]
    fn falls_back_to_global_default() {
        let dir = TempDir::new().unwrap();
        let res = resolve_anchor_version_with(dir.path(), &[], Some(v("0.30.0")))
            .unwrap()
            .unwrap();
        assert!(matches!(res.source, ResolutionSource::GlobalDefault));
        assert_eq!(res.version, v("0.30.0"));
    }

    #[test]
    fn returns_none_when_nothing_resolves() {
        let dir = TempDir::new().unwrap();
        let res = resolve_anchor_version_with(dir.path(), &[], None).unwrap();
        assert!(res.is_none());
    }

    #[test]
    fn precedence_anchor_toml_beats_anchorversion_and_cargo_toml() {
        let dir = TempDir::new().unwrap();
        write(
            &dir.path().join("Anchor.toml"),
            "[toolchain]\nanchor_version = \"0.30.1\"\n",
        );
        write(&dir.path().join(".anchorversion"), "0.28.0\n");
        write(
            &dir.path().join("programs/foo/Cargo.toml"),
            "[package]\nname = \"foo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[lib]\npath = \
             \"src/lib.rs\"\n[dependencies]\nanchor-lang = \"0.29.0\"\n",
        );
        write(&dir.path().join("programs/foo/src/lib.rs"), "");
        let res = resolve_anchor_version_with(dir.path(), &[], Some(v("0.27.0")))
            .unwrap()
            .unwrap();
        assert_eq!(res.version, v("0.30.1"));
        assert!(matches!(res.source, ResolutionSource::AnchorToml(_)));
    }
}
