//! `TestMultiAccount<Counter>` requires both `min_value` and
//! `bump_on_exit`. This usage attaches only one — should fail with the
//! diagnostic surfacing the missing `MinValueConstraint`.

use {
    anchor_lang_v2::prelude::*,
    custom_constraints::{counter_ns, Counter, TestMultiAccount},
};

#[derive(Accounts)]
pub struct Partial {
    #[account(
        mut,
        seeds = [b"x"],
        bump,
        counter_ns::bump_on_exit = 1u64,
    )]
    pub counter: TestMultiAccount<Counter>,
}

fn main() {}
