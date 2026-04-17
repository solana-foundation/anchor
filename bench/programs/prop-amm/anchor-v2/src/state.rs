use anchor_lang_v2::prelude::*;

/// Minimal on-chain oracle. `#[account]` gives us an 8-byte discriminator
/// prefix; the fields are laid out POD so the asm fast path can write
/// `price` at byte offset `8 + 32 = 40` without a full deserialize.
#[account]
pub struct Oracle {
    pub authority: Address,
    pub price: u64,
}
