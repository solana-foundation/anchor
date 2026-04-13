//! Associated Token Account address derivation.
//!
//! Users can validate ATA accounts via `constraint = expr`:
//! ```ignore
//! #[account(
//!     token::mint = mint,
//!     token::authority = authority,
//!     constraint = *vault.account().address() == anchor_spl_v2::get_associated_token_address(
//!         authority.account().address(),
//!         mint.account().address(),
//!         &Token::id(),
//!     )
//! )]
//! pub vault: Account<TokenAccount>,
//! ```

use solana_address::Address;

/// Derive the associated token account address for a given wallet, mint, and token program.
pub fn get_associated_token_address(
    wallet: &Address,
    mint: &Address,
    token_program_id: &Address,
) -> Address {
    let ata_program_id = Address::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
    let seeds: &[&[u8]] = &[wallet.as_ref(), token_program_id.as_ref(), mint.as_ref()];
    let (addr, _bump) = Address::find_program_address(seeds, &ata_program_id);
    addr
}
