//! `TestAccount<Counter>` requires `counter_ns::min_value`. This usage
//! omits it — should fail to compile with an `IsSuperset`/`Find` trait
//! bound error naming `MinValueConstraint`.

use {
    anchor_lang_v2::prelude::*,
    custom_constraints::{Counter, TestAccount},
};

#[derive(Accounts)]
pub struct Missing {
    #[account(seeds = [b"x"], bump)]
    pub counter: TestAccount<Counter>,
}

fn main() {}
