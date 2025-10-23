#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;

use clippy_utils::diagnostics::span_lint;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};

dylint_linting::declare_late_lint! {
    #[doc = include_str!("../README.md")]
    pub EXAMPLE_LINT,
    Warn,
    "use of `msg!(\"Hello, world!\")"
}

impl<'tcx> LateLintPass<'tcx> for ExampleLint {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(target, args) = expr.kind
            && let ExprKind::Path(QPath::Resolved(_, path)) = target.kind
            && let Some(func) = &path.segments.last()
            && func.ident.as_str() == "sol_log"
            && let [arg] = &args
            && let ExprKind::Lit(lit) = arg.kind
            && lit
                .node
                .str()
                .is_some_and(|s| s.as_str() == "Hello, world!")
        {
            span_lint(
                cx,
                EXAMPLE_LINT,
                expr.span.source_callsite(),
                "Use of `msg!(\"Hello, world!\")`",
            );
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
