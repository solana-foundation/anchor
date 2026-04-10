//! Trait-based constraint system for account validation.
//!
//! The `Constrain<C, V>` trait lets account types declare what constraint keys
//! they support. The `#[derive(Accounts)]` macro generates trait calls for
//! `namespace::key = expr` constraints. The marker type path resolves through
//! normal `use` imports — external crates provide their own markers.
//!
//! Adding a new constraint requires only:
//! 1. A marker struct in the appropriate namespace module below
//! 2. An `impl Constrain<marker>` on the relevant account type

use solana_program_error::ProgramError;

/// A constraint check on an account. Each account type opts in to specific
/// constraint keys by implementing this trait for the corresponding marker.
///
/// `V` is the expected value type — defaults to `Address` for address comparisons.
/// Use a different `V` for non-address checks (e.g. `mint::Decimals` uses `u8`).
///
/// Unknown keys → compile error ("Constrain<X> is not implemented for Y").
pub trait Constrain<C, V = solana_address::Address> {
    fn constrain(&self, expected: &V) -> Result<(), ProgramError>;
}

/// Constraint markers for SPL Token accounts.
pub mod token {
    pub struct Mint;
    pub struct Authority;
    pub struct TokenProgram;
}

/// Constraint markers for SPL Mint accounts.
pub mod mint {
    pub struct Authority;
    pub struct FreezeAuthority;
    pub struct Decimals;
    pub struct TokenProgram;
}
