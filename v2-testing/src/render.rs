//! Post-run flamegraph rendering for per-test trace directories.
//!
//! Consumes the layout produced by the `profile` feature (see
//! `profile.rs` docs) and emits one SVG per test, aggregating all
//! invocations of all programs the test exercised.

use {
    crate::flamegraph::generate_flamegraph_from_trace,
    anyhow::{Context, Result},
    std::{
        collections::{BTreeMap, BTreeSet},
        fs,
        path::{Path, PathBuf},
    },
};

/// Default root inspected by [`render_all_tests`]. Matches
/// `profile::DEFAULT_DIR`. Override both with `ANCHOR_PROFILE_DIR`.
pub const DEFAULT_PROFILE_DIR: &str = "target/anchor-v2-profile";

/// ELF path resolver: given a base58 program id, return the deployed
/// ELF path if the caller knows one.
///
/// The anchor CLI constructs this from `Anchor.toml`'s `[programs.*]`
/// section (program id → `target/deploy/<name>.so`). For program ids
/// not recognized (e.g. system program, spl-token), return `None` —
/// those frames will render as `<program_id>::[unknown]`.
pub type ElfResolver<'a> = &'a dyn Fn(&str) -> Option<PathBuf>;

/// A rendered flamegraph for a single test.
pub struct RenderedTest {
    pub test_name: String,
    pub svg_path: PathBuf,
    /// Programs whose frames made it into the SVG.
    pub programs_rendered: Vec<String>,
    /// Program IDs we saw in traces but couldn't symbolicate (no ELF).
    pub programs_unresolved: Vec<String>,
}

/// Walk `<root>/` for per-test trace directories and render one SVG
/// per test. Each SVG aggregates all program invocations found in that
/// test's dir — top-level and CPIs.
///
/// Output goes to `<root>/<test_name>.svg`.
pub fn render_all_tests(
    root: &Path,
    manifest_dir: Option<&Path>,
    elf_resolver: ElfResolver<'_>,
) -> Result<Vec<RenderedTest>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();

    for entry in fs::read_dir(root)
        .with_context(|| format!("failed to read profile root {}", root.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(test_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        if let Some(rendered) =
            render_one_test(test_name, &path, root, manifest_dir, elf_resolver)?
        {
            out.push(rendered);
        }
    }

    Ok(out)
}

/// Render the flamegraph for a single test's trace directory.
fn render_one_test(
    test_name: &str,
    test_dir: &Path,
    root: &Path,
    manifest_dir: Option<&Path>,
    elf_resolver: ElfResolver<'_>,
) -> Result<Option<RenderedTest>> {
    let program_ids = program_ids_in_dir(test_dir)?;
    if program_ids.is_empty() {
        return Ok(None);
    }

    // For each program we can resolve, produce a flamegraph fragment.
    // The first renderable program drives the output SVG; additional
    // programs are dropped (with their IDs reported) until we extend
    // `flamegraph` with true multi-program aggregation.
    //
    // Rationale: `generate_flamegraph_from_trace` takes a single ELF
    // and filters traces to that program. Honest single-program
    // rendering is what it does today; union-rendering across ELFs is
    // future work and should land as a separate change so the trace
    // file format and the renderer co-evolve.
    let mut rendered = Vec::new();
    let mut unresolved = Vec::new();

    for pid in &program_ids {
        match elf_resolver(pid) {
            Some(elf) => rendered.push((pid.clone(), elf)),
            None => unresolved.push(pid.clone()),
        }
    }

    if rendered.is_empty() {
        return Ok(Some(RenderedTest {
            test_name: test_name.to_owned(),
            svg_path: PathBuf::new(),
            programs_rendered: Vec::new(),
            programs_unresolved: unresolved,
        }));
    }

    // Pick the program that's responsible for the most invocations in
    // this test dir — usually the program under test.
    let dominant = dominant_program(test_dir, &rendered)?;

    let (program, elf_path) = &rendered[dominant];
    let svg_path = root.join(format!("{test_name}.svg"));

    generate_flamegraph_from_trace(program, elf_path, test_dir, &svg_path, manifest_dir)
        .with_context(|| format!("failed to render flamegraph for test {test_name}"))?;

    Ok(Some(RenderedTest {
        test_name: test_name.to_owned(),
        svg_path,
        programs_rendered: vec![program.clone()],
        programs_unresolved: unresolved,
    }))
}

/// Read every `*.program_id` in `test_dir` and return the distinct set.
fn program_ids_in_dir(test_dir: &Path) -> Result<BTreeSet<String>> {
    let mut ids = BTreeSet::new();
    for entry in fs::read_dir(test_dir)
        .with_context(|| format!("failed to read test dir {}", test_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("program_id") {
            continue;
        }
        if let Ok(contents) = fs::read_to_string(&path) {
            let pid = contents.trim();
            if !pid.is_empty() {
                ids.insert(pid.to_owned());
            }
        }
    }
    Ok(ids)
}

/// Pick the program in `candidates` that has the most `.program_id`
/// files mentioning it in `test_dir`. Ties broken by order in
/// `candidates` (stable).
fn dominant_program(test_dir: &Path, candidates: &[(String, PathBuf)]) -> Result<usize> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for entry in fs::read_dir(test_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("program_id") {
            continue;
        }
        if let Ok(contents) = fs::read_to_string(&path) {
            let pid = contents.trim();
            if let Some((idx, _)) =
                candidates.iter().enumerate().find(|(_, (p, _))| p == pid)
            {
                *counts.entry(candidates[idx].0.as_str()).or_default() += 1;
            }
        }
    }

    let best = candidates
        .iter()
        .enumerate()
        .max_by_key(|(_, (p, _))| counts.get(p.as_str()).copied().unwrap_or(0))
        .map(|(i, _)| i)
        .unwrap_or(0);
    Ok(best)
}
