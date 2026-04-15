//! Anchor benchmark framework — reusable library for running Solana program benchmarks.
//!
//! Provides `BenchContext` for setting up LiteSVM with a loaded program,
//! `BenchInstruction` for describing benchmark transactions, and helper
//! functions for executing and measuring them.
//!
//! Also owns the benchmark suite table (`SUITES`) and the end-to-end
//! `run()` orchestrator so that both the `anchor-bench` binary and
//! integration tests can drive the same flow.

pub mod bench;
pub mod flamegraph;
pub mod programs;

#[cfg(feature = "bin")]
pub mod history;

pub use bench::{
    execute_benchmark, execute_benchmark_with_tracing, keypair_for_account, BenchContext,
    BenchInstruction, CaseBuilder, InstructionSuite, ProgramSuite,
};
pub use flamegraph::generate_flamegraph_from_trace;
pub use flamegraph::print_ix_trace_to;

// ---------------------------------------------------------------------------
// Program id helpers and suite table
// ---------------------------------------------------------------------------

use {
    programs::{
        helloworld::{
            anchor_v1 as helloworld_anchor_v1, anchor_v2 as helloworld_anchor_v2,
            pinocchio as helloworld_pinocchio, quasar as helloworld_quasar,
            steel as helloworld_steel,
        },
        multisig::{
            anchor_v1, anchor_v2, pinocchio as multisig_pinocchio, quasar,
            steel as multisig_steel,
        },
        vault::{
            anchor_v2 as vault_anchor_v2, quasar as vault_quasar,
        },
    },
    solana_pubkey::Pubkey,
};

// hello_world: all 5 variants share the same id `B7ihZyo...` so cross-framework
// comparisons are apples-to-apples (no accidental PDA-bump differences). The
// shape is also identical: 1 unchecked read-only account (the payer).
const HELLO_WORLD_ID_STR: &str = "B7ihZyoXZ1fwAY3TugkiFJ6SXkzJwMuQrxrekBaSmn32";
fn hello_world_id() -> Pubkey { HELLO_WORLD_ID_STR.parse().unwrap() }

// multisig: all five variants (v1, v2, quasar, pinocchio, steel) share the
// same program id (`4444...4444`) so `find_program_address` returns the same
// bumps across variants, eliminating a confounding variable from the CU
// comparison.
fn multisig_shared_id() -> Pubkey {
    "44444444444444444444444444444444444444444444".parse().unwrap()
}

// vault: two variants (v2, quasar) sharing `3333...3333` — the same
// declare_id as the quasar-vault example we're copying verbatim.
fn vault_shared_id() -> Pubkey {
    "33333333333333333333333333333333333333333333".parse().unwrap()
}

/// Single source of truth for every benchmarked program and instruction.
///
/// The binary driver iterates this slice; integration tests filter it
/// (e.g. `suites_with_prefix("multisig")`) to run subsets.
pub const SUITES: &[ProgramSuite] = &[
    // 5-way counter init benchmark: every framework creates the same
    // `[b"counter"]` PDA, writes value = 42 + bump, using each one's idiomatic
    // init path. Pinocchio is hand-optimized as the performance floor.
    ProgramSuite {
        name: "hello_world",
        family: "hello_world",
        variant: "anchor v1",
        manifest_dir: "programs/helloworld/anchor-v1",
        instructions: &[
            InstructionSuite { name: "init", program_id: hello_world_id, build: helloworld_anchor_v1::build_init_case },
        ],
    },
    ProgramSuite {
        name: "hello_world_v2",
        family: "hello_world",
        variant: "anchor v2",
        manifest_dir: "programs/helloworld/anchor-v2",
        instructions: &[
            InstructionSuite { name: "init", program_id: hello_world_id, build: helloworld_anchor_v2::build_init_case },
        ],
    },
    ProgramSuite {
        name: "hello_world_quasar",
        family: "hello_world",
        variant: "quasar",
        manifest_dir: "programs/helloworld/quasar",
        instructions: &[
            InstructionSuite { name: "init", program_id: hello_world_id, build: helloworld_quasar::build_init_case },
        ],
    },
    ProgramSuite {
        name: "hello_world_pinocchio",
        family: "hello_world",
        variant: "pinocchio",
        manifest_dir: "programs/helloworld/pinocchio",
        instructions: &[
            InstructionSuite { name: "init", program_id: hello_world_id, build: helloworld_pinocchio::build_init_case },
        ],
    },
    ProgramSuite {
        name: "hello_world_steel",
        family: "hello_world",
        variant: "steel",
        manifest_dir: "programs/helloworld/steel",
        instructions: &[
            InstructionSuite { name: "init", program_id: hello_world_id, build: helloworld_steel::build_init_case },
        ],
    },
    ProgramSuite {
        name: "multisig_v1",
        family: "multisig",
        variant: "anchor v1",
        manifest_dir: "programs/multisig/anchor-v1",
        instructions: &[
            InstructionSuite { name: "create",           program_id: multisig_shared_id, build: anchor_v1::build_create_case },
            InstructionSuite { name: "deposit",          program_id: multisig_shared_id, build: anchor_v1::build_deposit_case },
            InstructionSuite { name: "set_label",        program_id: multisig_shared_id, build: anchor_v1::build_set_label_case },
            InstructionSuite { name: "execute_transfer", program_id: multisig_shared_id, build: anchor_v1::build_execute_transfer_case },
        ],
    },
    ProgramSuite {
        name: "multisig_v2",
        family: "multisig",
        variant: "anchor v2",
        manifest_dir: "programs/multisig/anchor-v2",
        instructions: &[
            InstructionSuite { name: "create",           program_id: multisig_shared_id, build: anchor_v2::build_create_case },
            InstructionSuite { name: "deposit",          program_id: multisig_shared_id, build: anchor_v2::build_deposit_case },
            InstructionSuite { name: "set_label",        program_id: multisig_shared_id, build: anchor_v2::build_set_label_case },
            InstructionSuite { name: "execute_transfer", program_id: multisig_shared_id, build: anchor_v2::build_execute_transfer_case },
        ],
    },
    ProgramSuite {
        name: "multisig_quasar",
        family: "multisig",
        variant: "quasar",
        manifest_dir: "programs/multisig/quasar",
        instructions: &[
            InstructionSuite { name: "create",           program_id: multisig_shared_id, build: quasar::build_create_case },
            InstructionSuite { name: "deposit",          program_id: multisig_shared_id, build: quasar::build_deposit_case },
            InstructionSuite { name: "set_label",        program_id: multisig_shared_id, build: quasar::build_set_label_case },
            InstructionSuite { name: "execute_transfer", program_id: multisig_shared_id, build: quasar::build_execute_transfer_case },
        ],
    },
    ProgramSuite {
        name: "multisig_pinocchio",
        family: "multisig",
        variant: "pinocchio",
        manifest_dir: "programs/multisig/pinocchio",
        instructions: &[
            InstructionSuite { name: "create",           program_id: multisig_shared_id, build: multisig_pinocchio::build_create_case },
            InstructionSuite { name: "deposit",          program_id: multisig_shared_id, build: multisig_pinocchio::build_deposit_case },
            InstructionSuite { name: "set_label",        program_id: multisig_shared_id, build: multisig_pinocchio::build_set_label_case },
            InstructionSuite { name: "execute_transfer", program_id: multisig_shared_id, build: multisig_pinocchio::build_execute_transfer_case },
        ],
    },
    ProgramSuite {
        name: "multisig_steel",
        family: "multisig",
        variant: "steel",
        manifest_dir: "programs/multisig/steel",
        instructions: &[
            InstructionSuite { name: "create",           program_id: multisig_shared_id, build: multisig_steel::build_create_case },
            InstructionSuite { name: "deposit",          program_id: multisig_shared_id, build: multisig_steel::build_deposit_case },
            InstructionSuite { name: "set_label",        program_id: multisig_shared_id, build: multisig_steel::build_set_label_case },
            InstructionSuite { name: "execute_transfer", program_id: multisig_shared_id, build: multisig_steel::build_execute_transfer_case },
        ],
    },
    // 2-way quasar-vault benchmark: a minimal SOL vault with deposit
    // (system::transfer CPI) and withdraw (direct lamport arithmetic).
    // The quasar variant is copied verbatim from
    // `the quasar vault example`; the v2 variant is a shape-matched
    // port. Only these two variants for now — v1 / pinocchio / steel
    // can be added later if useful for direct comparison.
    ProgramSuite {
        name: "vault_v2",
        family: "vault",
        variant: "anchor v2",
        manifest_dir: "programs/vault/anchor-v2",
        instructions: &[
            InstructionSuite { name: "deposit",  program_id: vault_shared_id, build: vault_anchor_v2::build_deposit_case },
            InstructionSuite { name: "withdraw", program_id: vault_shared_id, build: vault_anchor_v2::build_withdraw_case },
        ],
    },
    ProgramSuite {
        name: "vault_quasar",
        family: "vault",
        variant: "quasar",
        manifest_dir: "programs/vault/quasar",
        instructions: &[
            InstructionSuite { name: "deposit",  program_id: vault_shared_id, build: vault_quasar::build_deposit_case },
            InstructionSuite { name: "withdraw", program_id: vault_shared_id, build: vault_quasar::build_withdraw_case },
        ],
    },
];

/// Returns a vec of suites whose `name` starts with `prefix`.
/// Useful for integration tests that want to run a subset.
pub fn suites_with_prefix(prefix: &str) -> Vec<ProgramSuite> {
    SUITES.iter().copied().filter(|s| s.name.starts_with(prefix)).collect()
}

/// Prints a pretty CU + binary-size comparison table for all programs in
/// `family` that implement the given `instruction`, sorted by CU ascending.
///
/// ```text
/// multisig/create
///   rank  variant      bytes      CU   vs best
///   ----  ---------  -------  ------  --------
///      1  anchor v2   30,712   2,960
///      2  quasar      30,928   4,195   +1,235
/// ```
#[cfg(feature = "bin")]
pub fn print_instruction_comparison(
    result: &crate::history::BenchmarkResult,
    family: &str,
    instruction: &str,
) {
    // Collect the (suite, cu, binary) rows for this family + instruction.
    let mut rows: Vec<(&ProgramSuite, u64, u64)> = SUITES
        .iter()
        .filter(|s| s.family == family)
        .filter_map(|suite| {
            let program = result.programs.get(suite.name)?;
            let cu = *program.compute_units.get(instruction)?;
            Some((suite, cu, program.binary_size_bytes))
        })
        .collect();

    if rows.is_empty() {
        return;
    }

    rows.sort_by_key(|(_, cu, _)| *cu);
    let best_cu = rows[0].1;

    // Width-fit the variant column.
    let variant_w = rows.iter().map(|(s, _, _)| s.variant.len()).max().unwrap_or(0).max(7);

    println!();
    println!("{}/{}", family, instruction);
    println!(
        "  {:>4}  {:<variant_w$}  {:>9}  {:>7}  {:>8}",
        "rank", "variant", "bytes", "CU", "vs best"
    );
    println!(
        "  {}  {}  {}  {}  {}",
        "-".repeat(4),
        "-".repeat(variant_w),
        "-".repeat(9),
        "-".repeat(7),
        "-".repeat(8),
    );
    for (rank, (suite, cu, bytes)) in rows.iter().enumerate() {
        let delta = if rank == 0 {
            String::new()
        } else {
            format!("+{}", format_with_commas(cu - best_cu))
        };
        println!(
            "  {:>4}  {:<variant_w$}  {:>9}  {:>7}  {:>8}",
            rank + 1,
            suite.variant,
            format_with_commas(*bytes),
            format_with_commas(*cu),
            delta,
        );
    }
}

/// Formats a number with thousands-separator commas, e.g. `12345` → `"12,345"`.
#[cfg(feature = "bin")]
fn format_with_commas(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

// ---------------------------------------------------------------------------
// Run orchestration
// ---------------------------------------------------------------------------

#[cfg(feature = "bin")]
mod run {
    use {
        super::{
            bench::{build_programs, build_results, execute_benchmark_with_tracing, ProgramSuite},
            flamegraph::generate_flamegraph_from_trace,
            history::BenchmarkResult,
        },
        anyhow::Result,
        std::path::Path,
    };

    /// Options for [`run`].
    #[derive(Clone, Copy, Default)]
    pub struct RunOptions {
        /// Skip the `cargo build-sbf` step and use pre-built `target/deploy/*.so`.
        pub skip_build: bool,
        /// After measurement, regenerate per-instruction flamegraphs.
        pub flamegraphs: bool,
    }

    /// End-to-end benchmark driver. Builds the programs (unless skipped),
    /// measures binary sizes and compute units, and optionally generates
    /// flamegraphs. Returns the measured result for the caller to persist,
    /// compare, or assert against.
    ///
    /// Also honours `BENCH_IX_TRACE=<suite>/<instruction>` (e.g.
    /// `hello_world_v2/init`) to dump a full per-instruction register
    /// trace for one benchmark to `bench/ix_traces/<label>.trace`. Useful
    /// for dynamic-CU analysis — see `flamegraph::print_ix_trace_to`.
    pub fn run(
        bench_dir: &Path,
        suites: &[ProgramSuite],
        options: RunOptions,
    ) -> Result<BenchmarkResult> {
        if !options.skip_build {
            build_programs(bench_dir, suites)?;
        }
        let result = build_results(bench_dir, suites)?;
        if options.flamegraphs {
            generate_flamegraphs(bench_dir, suites);
        }
        if let Ok(target) = std::env::var("BENCH_IX_TRACE") {
            generate_ix_trace(bench_dir, suites, &target);
        }
        Ok(result)
    }

    /// Looks up a single `<suite>/<instruction>` target (as specified by
    /// `BENCH_IX_TRACE`), runs that benchmark with register tracing enabled,
    /// and dumps a per-instruction trace to
    /// `bench/ix_traces/<suite>_<instruction>.trace`.
    fn generate_ix_trace(bench_dir: &Path, suites: &[ProgramSuite], target: &str) {
        let Some((suite_name, ix_name)) = target.split_once('/') else {
            eprintln!(
                "BENCH_IX_TRACE={target:?} is not in <suite>/<instruction> form; skipping"
            );
            return;
        };
        let Some(suite) = suites.iter().find(|s| s.name == suite_name) else {
            eprintln!("BENCH_IX_TRACE: no suite named {suite_name:?}");
            return;
        };
        let Some(instruction) = suite.instructions.iter().find(|i| i.name == ix_name) else {
            eprintln!(
                "BENCH_IX_TRACE: suite {suite_name:?} has no instruction {ix_name:?}"
            );
            return;
        };

        let so_path = bench_dir
            .join("../target/deploy")
            .join(format!("{}.so", suite.name));
        if !so_path.exists() {
            eprintln!(
                "BENCH_IX_TRACE: deployed ELF missing at {}",
                so_path.display()
            );
            return;
        }

        let label = format!("{}_{}", suite.name, instruction.name);
        println!("Generating ix trace for {label}...");
        let trace_dir = match execute_benchmark_with_tracing(
            &so_path,
            (instruction.program_id)(),
            instruction.build,
        ) {
            Ok(dir) => dir,
            Err(err) => {
                eprintln!("Warning: tracing run for {label} failed: {err:#}");
                return;
            }
        };

        let out_dir = bench_dir.join("ix_traces");
        if let Err(err) = std::fs::create_dir_all(&out_dir) {
            eprintln!(
                "Warning: could not create {}: {err:#}",
                out_dir.display()
            );
            return;
        }
        let out_path = out_dir.join(format!("{label}.trace"));
        let mut file = match std::fs::File::create(&out_path) {
            Ok(f) => f,
            Err(err) => {
                eprintln!("Warning: could not open {}: {err:#}", out_path.display());
                return;
            }
        };
        let manifest_dir = bench_dir.join(suite.manifest_dir);
        match super::flamegraph::print_ix_trace_to(
            &mut file,
            &label,
            &so_path,
            trace_dir.path(),
            Some(&manifest_dir),
        ) {
            Ok(()) => println!("  Ix trace: {}", out_path.display()),
            Err(err) => eprintln!("  Warning: ix trace for {label} failed: {err:#}"),
        }
    }

    fn generate_flamegraphs(bench_dir: &Path, suites: &[ProgramSuite]) {
        let flamegraph_dir = bench_dir.join("flamegraphs");
        for suite in suites {
            let so_path = bench_dir
                .join("../target/deploy")
                .join(format!("{}.so", suite.name));
            if !so_path.exists() {
                continue;
            }
            for instruction in suite.instructions {
                let label = format!("{}_{}", suite.name, instruction.name);
                println!("Generating flamegraph for {label}...");
                let trace_dir = match execute_benchmark_with_tracing(
                    &so_path,
                    (instruction.program_id)(),
                    instruction.build,
                ) {
                    Ok(dir) => dir,
                    Err(err) => {
                        eprintln!("Warning: tracing run for {label} failed: {err:#}");
                        continue;
                    }
                };
                let svg_path = flamegraph_dir.join(format!("{label}.svg"));
                let manifest_dir = bench_dir.join(suite.manifest_dir);
                match generate_flamegraph_from_trace(
                    &label,
                    &so_path,
                    trace_dir.path(),
                    &svg_path,
                    Some(&manifest_dir),
                ) {
                    Ok(()) => println!("  Flamegraph: {}", svg_path.display()),
                    Err(err) => eprintln!("  Warning: flamegraph for {label} failed: {err:#}"),
                }
            }
        }
    }
}

#[cfg(feature = "bin")]
pub use run::{run, RunOptions};
