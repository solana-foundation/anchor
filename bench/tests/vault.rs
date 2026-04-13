//! End-to-end benchmark test for the vault family (2 framework variants).
//!
//! The quasar variant is an **exact copy** of
//! `~/git/quasar/examples/vault/src/` so the comparison measures the
//! hand-tuned quasar-lang implementation; the v2 variant is a shape-
//! matched port using anchor-lang-v2. Both expose two instructions:
//! `deposit` (SOL via system::Transfer CPI) and `withdraw` (direct
//! lamport arithmetic).

use {
    anchor_bench::{print_instruction_comparison, run, suites_with_prefix, RunOptions},
    std::path::PathBuf,
};

const EXPECTED_SUITES: &[&str] = &["vault_v2", "vault_quasar"];
const INSTRUCTIONS: &[&str] = &["deposit", "withdraw"];

#[test]
fn vault_end_to_end() {
    let bench_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suites = suites_with_prefix("vault");
    assert_eq!(
        suites.len(),
        EXPECTED_SUITES.len(),
        "expected {} vault suites, got {}",
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
        for &ix in INSTRUCTIONS {
            let cu = program
                .compute_units
                .get(ix)
                .unwrap_or_else(|| panic!("missing CU for {suite_name}/{ix}"));
            assert!(*cu > 0, "{suite_name}/{ix} reports 0 CU");
            assert!(
                *cu < 20_000,
                "{suite_name}/{ix} CU {cu} exceeds sanity cap"
            );
        }
    }

    for &ix in INSTRUCTIONS {
        print_instruction_comparison(&result, "vault", ix);
    }
}
