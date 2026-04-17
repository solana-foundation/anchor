use anchor_lang_v2::prelude::*;

#[account]
pub struct Oracle {
    pub authority: Address,
    pub price: u64,
}
