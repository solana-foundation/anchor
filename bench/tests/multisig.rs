//! End-to-end benchmark test for the multisig family (v1 + v2).

use {
    anchor_bench::{print_instruction_comparison, run, suites_with_prefix, RunOptions},
    std::path::PathBuf,
};

const EXPECTED_SUITES: &[&str] = &["multisig_v1", "multisig_v2"];
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
