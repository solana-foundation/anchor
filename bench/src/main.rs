mod graphs;

use {
    anchor_bench::{
        history::{
            load_history, save_history, update_history, validate_history_commits, RESULTS_FILE,
        },
        run, RunOptions, SUITES,
    },
    crate::graphs::{render_graphs, GRAPHS_DIR},
    anyhow::{bail, Result},
    std::{env, path::PathBuf},
};

/// Controls whether the benchmark run updates the history file or only validates it.
enum RunMode {
    Record,
    Check,
}

struct CliArgs {
    mode: RunMode,
    skip_build: bool,
}

impl CliArgs {
    fn from_args() -> Result<Self> {
        let mut mode = RunMode::Record;
        let mut skip_build = false;
        for arg in env::args().skip(1) {
            match arg.as_str() {
                "check" | "--check" => mode = RunMode::Check,
                "--skip-build" => skip_build = true,
                other => bail!("unsupported anchor-bench argument: {other}"),
            }
        }
        Ok(Self { mode, skip_build })
    }
}

/// Builds benchmark programs, runs the configured suites, and updates `results.json`.
fn main() -> Result<()> {
    let cli = CliArgs::from_args()?;
    let bench_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let current_result = run(
        &bench_dir,
        SUITES,
        RunOptions {
            skip_build: cli.skip_build,
            flamegraphs: true,
        },
    )?;

    let results_path = bench_dir.join(RESULTS_FILE);
    let history = load_history(&results_path)?;
    let mut updated_history = history.clone();

    update_history(&mut updated_history, current_result)?;
    validate_history_commits(&updated_history)?;

    match cli.mode {
        RunMode::Record => {
            save_history(&results_path, &updated_history)?;
            render_graphs(&bench_dir, &updated_history)?;

            println!("Stored benchmark results in {}", results_path.display());
            println!(
                "Stored benchmark graphs in {}",
                bench_dir.join(GRAPHS_DIR).display()
            );
        }
        RunMode::Check => {
            if history != updated_history {
                bail!(
                    "benchmarks have changed without being recorded in {}. Run `cargo run \
                     --manifest-path bench/Cargo.toml --locked` to refresh them.",
                    results_path.display()
                );
            }

            println!("Benchmark history is up to date");
        }
    }

    Ok(())
}
