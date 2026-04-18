//! DWARF-backed `PC → (file, line)` resolver for SBF ELFs.
//!
//! Built on [`addr2line::Loader`] so we get DWARF-5 / split-DWARF / supplementary
//! object support for free. The resolver is best-effort: if the ELF has no
//! DWARF (stripped deploys, release builds without
//! `CARGO_PROFILE_RELEASE_DEBUG=2`, programs built by third parties) we return
//! `None` and the TUI source pane falls back to a "no source available" notice
//! — the rest of the stepper is unaffected.
//!
//! SBF maps PCs to byte addresses as `text_addr + pc * INSN_SIZE`. LLVM emits
//! standard DWARF line tuples on those byte addresses so `addr2line` works
//! once we do that translation.

use addr2line::Loader;
use object::{Object, ObjectSection};
use std::path::{Path, PathBuf};

use super::model::SrcLoc;

const INSN_SIZE: u64 = 8;

/// Per-ELF source resolver. Cheap to query (one interval-tree lookup).
pub struct SourceResolver {
    inner: Option<Inner>,
}

struct Inner {
    loader: Loader,
    text_addr: u64,
}

impl SourceResolver {
    /// Builds a resolver by re-reading the ELF from disk. Returns an empty
    /// resolver when parsing fails or no text section is present.
    pub fn from_elf_path(path: &Path) -> Self {
        Self {
            inner: build(path).ok(),
        }
    }

    /// Resolves an SBPF program counter to a `(file, line)` pair, or `None`
    /// when DWARF is unavailable or the PC has no line entry.
    pub fn resolve(&self, pc: u64) -> Option<SrcLoc> {
        let inner = self.inner.as_ref()?;
        let vaddr = inner.text_addr.checked_add(pc.checked_mul(INSN_SIZE)?)?;
        let loc = inner.loader.find_location(vaddr).ok().flatten()?;
        Some(SrcLoc {
            file: PathBuf::from(loc.file?),
            line: loc.line?,
        })
    }

    /// `true` when no DWARF context was built — the TUI uses this to render
    /// a single "no source info" notice instead of per-step errors.
    pub fn is_empty(&self) -> bool {
        self.inner.is_none()
    }
}

/// Prefix the Solana `platform-tools` build bakes into DWARF paths for
/// stdlib files.
#[cfg(target_os = "macos")]
pub const CI_PLATFORM_TOOLS_PREFIX: &str =
    "/Users/runner/work/platform-tools/platform-tools/out/rust/library/";
#[cfg(target_os = "linux")]
pub const CI_PLATFORM_TOOLS_PREFIX: &str =
    "/home/runner/work/platform-tools/platform-tools/out/rust/library/";
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub const CI_PLATFORM_TOOLS_PREFIX: &str =
    compile_error!("Current platform is not supported");

/// Locate every `platform-tools/rust/lib/rustlib/src/rust/library/` tree
/// under `~/.cache/solana/` and return them newest-version-first. Empty
/// vec if the solana toolchain isn't installed via `agave-install`.
///
/// Callers pair these with [`CI_PLATFORM_TOOLS_PREFIX`] so stdlib frames
/// emitted against the CI build path resolve to the local source tree.
pub fn discover_platform_tools_stdlib_roots() -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let base = home.join(".cache/solana");
    let Ok(entries) = std::fs::read_dir(&base) else {
        return Vec::new();
    };
    let mut versions: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_owned();
            let candidate = e
                .path()
                .join("platform-tools/rust/lib/rustlib/src/rust/library");
            candidate.is_dir().then_some((name, candidate))
        })
        .collect();
    // Lexical sort on version strings like `v1.41`, `v1.52` — good enough
    // for numeric-minor ordering up to v1.99; beyond that we'd want proper
    // semver parsing.
    versions.sort_by(|a, b| b.0.cmp(&a.0));
    versions.into_iter().map(|(_, p)| p).collect()
}

fn build(path: &Path) -> anyhow::Result<Inner> {
    // Load once to read `.text` address. This parse is cheap (metadata only).
    let bytes = std::fs::read(path)?;
    let file = object::File::parse(&*bytes)?;
    let text = file
        .sections()
        .find(|s| s.name().ok() == Some(".text"))
        .or_else(|| {
            file.sections()
                .find(|s| s.kind() == object::SectionKind::Text)
        })
        .ok_or_else(|| anyhow::anyhow!("no .text section"))?;
    let text_addr = text.address();

    let loader = Loader::new(path).map_err(|e| anyhow::anyhow!("load DWARF: {e}"))?;
    Ok(Inner { loader, text_addr })
}
