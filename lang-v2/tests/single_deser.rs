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

fn _no_args_maps_to_unit() {
    let _: <NoArgs as TryAccounts>::IxArgs<'static> = ();
}

#[test]
fn no_instruction_args_maps_to_unit() {
    _no_args_maps_to_unit();
}

fn _instruction_args_are_tuple<'a>(args: <WithArgs as TryAccounts>::IxArgs<'a>) -> (u64, i32) {
    args
}

#[test]
fn instruction_args_map_to_tuple() {
    let _: fn(_) -> _ = _instruction_args_are_tuple;
}
