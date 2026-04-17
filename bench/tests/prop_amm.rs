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

#[test]
fn prop_amm_end_to_end() {
    let bench_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suites = suites_with_prefix("prop_amm");
    assert_eq!(suites.len(), 1, "expected exactly 1 prop_amm suite");

    let result = run(
        &bench_dir,
        &suites,
        RunOptions {
            skip_build: std::env::var("BENCH_SKIP_BUILD").is_ok(),
        },
    )
    .expect("bench run failed");

    let program = result
        .programs
        .get("prop_amm_v2")
        .expect("prop_amm_v2 missing from result");
    assert!(program.binary_size_bytes > 0, "prop_amm_v2 .so has zero bytes");

    // Envelopes derived from local measurements (initialize ~1.4k, update
    // ~40, rotate_authority ~100). Generous headroom so the check doesn't
    // oscillate on Solana SDK bumps; the update cap is tight enough that a
    // regression back into the dispatcher would trip it.
    let caps: &[(&str, u64)] = &[
        ("initialize", 10_000),
        ("update", 150),
        ("rotate_authority", 5_000),
    ];
    for &(name, cap) in caps {
        let cu = program
            .compute_units
            .get(name)
            .unwrap_or_else(|| panic!("missing CU for prop_amm_v2/{name}"));
        assert!(*cu > 0, "prop_amm_v2/{name} reports 0 CU");
        assert!(*cu < cap, "prop_amm_v2/{name} CU {cu} exceeds cap {cap}");
    }

    print_instruction_comparison(&result, "prop_amm", "initialize");
    print_instruction_comparison(&result, "prop_amm", "update");
    print_instruction_comparison(&result, "prop_amm", "rotate_authority");
}
