//! End-to-end benchmark test for the counter helloworld (5 framework variants).
//!
//! Builds each BPF program, runs the `init` instruction, asserts that the
//! compute-unit measurement produced a sane result, and prints a sorted
//! comparison table across all 5 frameworks. Set `BENCH_FLAMEGRAPHS=1` to
//! also regenerate flamegraphs, `BENCH_SKIP_BUILD=1` to reuse prebuilt .so.

use {
    anchor_bench::{print_instruction_comparison, run, suites_with_prefix, RunOptions},
    std::path::PathBuf,
};

const EXPECTED_SUITES: &[&str] = &[
    "hello_world",
    "hello_world_v2",
    "hello_world_quasar",
    "hello_world_pinocchio",
    "hello_world_steel",
];

#[test]
fn helloworld_end_to_end() {
    let bench_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suites = suites_with_prefix("hello_world");
    assert_eq!(
        suites.len(),
        EXPECTED_SUITES.len(),
        "expected {} hello_world suites, got {}",
        EXPECTED_SUITES.len(),
        suites.len()
    );

    let result = run(
        &bench_dir,
        &suites,
        RunOptions {
            skip_build: std::env::var("BENCH_SKIP_BUILD").is_ok(),
            flamegraphs: std::env::var("BENCH_FLAMEGRAPHS").is_ok(),
        },
    )
    .expect("bench run failed");

    for &suite_name in EXPECTED_SUITES {
        let program = result
            .programs
            .get(suite_name)
            .unwrap_or_else(|| panic!("missing {suite_name} in result"));
        assert!(
            program.binary_size_bytes > 0,
            "{suite_name} .so has zero bytes"
        );
        let cu = program
            .compute_units
            .get("init")
            .unwrap_or_else(|| panic!("missing CU for {suite_name}/init"));
        assert!(*cu > 0, "{suite_name}/init reports 0 CU");
        assert!(
            *cu < 20_000,
            "{suite_name}/init CU {cu} exceeds sanity cap"
        );
    }

    print_instruction_comparison(&result, "hello_world", "init");
}
