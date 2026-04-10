//! Trait-based constraint system for account validation.
//!
//! The `Constrain<C>` trait lets account types declare what constraint keys
//! they support. The `#[derive(Accounts)]` macro generates trait calls for
//! `namespace::key = expr` constraints without hardcoded knowledge of what
//! constraints exist. Unknown keys produce compile errors.
//!
//! Adding a new constraint requires only:
//! 1. A marker struct in the appropriate namespace module below
//! 2. An `impl Constrain<marker>` on the relevant account type

use {
    solana_address::Address,
    solana_program_error::ProgramError,
};

/// A constraint check on an account. Each account type opts in to specific
/// constraint keys by implementing this trait for the corresponding marker.
///
/// Unknown keys → compile error ("Constrain<X> is not implemented for Y").
pub trait Constrain<C> {
    fn constrain(&self, expected: &Address) -> Result<(), ProgramError>;
}

/// Constraint markers for SPL Token accounts.
pub mod token {
    /// Validates `token::mint = <account>` — the token account's mint matches.
    pub struct Mint;
    /// Validates `token::authority = <account>` — the token account's owner matches.
    pub struct Authority;
    /// Validates `token::token_program = <account>` — placeholder for token program check.
    pub struct TokenProgram;
}

/// Constraint markers for SPL Mint accounts.
pub mod mint {
    /// Validates `mint::authority = <account>` — the mint's mint_authority matches.
    pub struct Authority;
    /// Validates `mint::freeze_authority = <account>` — the mint's freeze_authority matches.
    pub struct FreezeAuthority;
}
