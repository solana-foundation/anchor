//! Compile-fail tests for `AnchorAccount::RequiredConstraints` enforcement.
//!
//! Each `compile_fail_*.rs` file under `ui_required_constraints/` should
//! fail to compile with a stable error mentioning either `IsSuperset` or
//! `Find` and the name of the missing constraint marker. Stderr snapshots
//! are committed alongside each case.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui_required_constraints/compile_fail_*.rs");
}
