//! End-to-end benchmark test for the `prop_amm` oracle asm-entrypoint demo.
//!
//! Builds the BPF program, runs all three instructions, and asserts CUs
//! land in sane envelopes. `update` is the whole point of this program:
//! an asm entrypoint that branches on the discriminator and short-circuits
//! price writes without ever entering the anchor dispatcher.

use {
    anchor_bench::{print_instruction_comparison, run, suites_with_prefix, RunOptions},
    std::path::PathBuf,
};

const EXPECTED_SUITES: &[&str] = &["prop_amm_v1", "prop_amm_v2"];
const INSTRUCTIONS: &[&str] = &["initialize", "update", "rotate_authority"];

#[test]
fn prop_amm_end_to_end() {
    let bench_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suites = suites_with_prefix("prop_amm");
    assert_eq!(
        suites.len(),
        EXPECTED_SUITES.len(),
        "expected {} prop_amm suites, got {}",
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
        for &ix in INSTRUCTIONS {
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

    // v2 `update` is the asm fast-path — tight cap so a regression back into
    // the dispatcher trips it.
    let v2 = result.programs.get("prop_amm_v2").expect("prop_amm_v2");
    let update_cu = v2.compute_units.get("update").expect("v2 update cu");
    assert!(
        *update_cu < 150,
        "prop_amm_v2/update CU {update_cu} exceeds fast-path cap 150"
    );

    for &ix in INSTRUCTIONS {
        print_instruction_comparison(&result, "prop_amm", ix);
    }
}
