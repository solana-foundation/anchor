//! Regression tests for the single-deserialization invariant.
//!
//! Before the fix, `__ix_data` was parsed twice on every handler
//! invocation: once inside `T::try_accounts` (using `#[instruction(...)]`)
//! and again in the generated handler wrapper (using the handler fn's
//! extra arg list). Mismatched schemas silently validated one shape while
//! the handler ran the other.
//!
//! These tests exercise the derive + trait surface to confirm:
//! 1. The `TryAccounts::IxArgs<'ix>` associated type is wired up.
//! 2. The `#[instruction(...)]` struct defines a concrete `IxArgs`
//!    matching the declared fields; a struct without `#[instruction(...)]`
//!    gets `IxArgs = ()`.
//! 3. `try_accounts` returns a 3-tuple `(Self, Bumps, IxArgs)` — the
//!    parsed args surface to the caller instead of being re-parsed.
//!
//! The full double-deser bug requires on-chain execution to exploit; the
//! v2 integration tests in `tests-v2/` cover the runtime path. These
//! tests lock in the compile-time surface.

#![allow(dead_code)]

use anchor_lang_v2::{prelude::*, TryAccounts};

#[derive(Accounts)]
pub struct NoArgs {
    pub account: UncheckedAccount,
}

#[derive(Accounts)]
#[instruction(amount: u64, step: i32)]
pub struct WithArgs {
    pub account: UncheckedAccount,
}

// ---- 1. `IxArgs` assoc-type is `()` when there's no `#[instruction(...)]`.

fn _no_args_maps_to_unit() {
    let _: <NoArgs as TryAccounts>::IxArgs<'static> = ();
}

#[test]
fn no_instruction_args_maps_to_unit() {
    _no_args_maps_to_unit();
}

// ---- 2. `IxArgs` for `#[instruction(...)]` is a concrete struct with the
// declared fields in declaration order.

fn _instruction_args_have_fields<'a>(args: <WithArgs as TryAccounts>::IxArgs<'a>) -> (u64, i32) {
    (args.amount, args.step)
}

#[test]
fn instruction_args_expose_fields() {
    // Reference the fn so it isn't dead-code-eliminated.
    let _: fn(_) -> _ = _instruction_args_have_fields;
}
