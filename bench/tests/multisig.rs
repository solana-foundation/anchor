//! End-to-end benchmark test for all five multisig program variants
//! (Anchor v1, Anchor v2, Quasar, Pinocchio, Steel).
//!
//! Builds each BPF program, runs every instruction, asserts the compute-unit
//! measurement produced a sane result, and prints a sorted comparison table
//! per instruction across all variants. Set `BENCH_FLAMEGRAPHS=1` to also
//! regenerate flamegraphs, `BENCH_SKIP_BUILD=1` to reuse prebuilt .so.

use {
    anchor_bench::{print_instruction_comparison, run, suites_with_prefix, RunOptions},
    std::path::PathBuf,
};

const EXPECTED_SUITES: &[&str] = &[
    "multisig_v1",
    "multisig_v2",
    "multisig_quasar",
    "multisig_pinocchio",
    "multisig_steel",
];
const EXPECTED_INSTRUCTIONS: &[&str] =
    &["create", "deposit", "set_label", "execute_transfer"];

#[test]
fn multisig_end_to_end() {
    let bench_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suites = suites_with_prefix("multisig");
    assert_eq!(
        suites.len(),
        EXPECTED_SUITES.len(),
        "expected {} multisig suites, got {}",
        EXPECTED_SUITES.len(),
        suites.len()
    );

    let result = run(
        &bench_dir,
        &suites,
        RunOptions {
            skip_build: std::env::var("BENCH_SKIP_BUILD").is_ok(),
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
        for &ix in EXPECTED_INSTRUCTIONS {
            let cu = program
                .compute_units
                .get(ix)
                .unwrap_or_else(|| panic!("missing CU for {suite_name}/{ix}"));
            assert!(*cu > 0, "{suite_name}/{ix} reports 0 CU");
            assert!(
                *cu < 100_000,
                "{suite_name}/{ix} CU {cu} exceeds sanity cap"
            );
        }
    }

    for &ix in EXPECTED_INSTRUCTIONS {
        print_instruction_comparison(&result, "multisig", ix);
    }
}
