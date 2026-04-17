use {
    anchor_bench::{print_instruction_comparison, run, suites_with_prefix, RunOptions},
    std::path::PathBuf,
};

const EXPECTED_SUITES: &[&str] = &["nested_v1", "nested_v2"];

#[test]
fn nested_end_to_end() {
    let bench_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suites = suites_with_prefix("nested");
    assert_eq!(
        suites.len(),
        EXPECTED_SUITES.len(),
        "expected {} nested suites, got {}",
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
        for ix_name in &["initialize", "increment", "reset"] {
            let cu = program
                .compute_units
                .get(*ix_name)
                .unwrap_or_else(|| panic!("missing CU for {suite_name}/{ix_name}"));
            assert!(*cu > 0, "{suite_name}/{ix_name} reports 0 CU");
        }
    }

    print_instruction_comparison(&result, "nested", "initialize");
    print_instruction_comparison(&result, "nested", "increment");
    print_instruction_comparison(&result, "nested", "reset");
}
