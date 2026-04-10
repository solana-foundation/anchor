//! Trait-based constraint system for account validation.
//!
//! The `Constrain<C>` trait lets account types declare what constraint keys
//! they support. The `#[derive(Accounts)]` macro generates trait calls for
//! `namespace::key = expr` constraints. The marker type path resolves through
//! normal `use` imports — external crates provide their own markers.
//!
//! Adding a new constraint requires only:
//! 1. A marker struct in a module (e.g. `anchor_spl::token::Mint`)
//! 2. An `impl Constrain<marker>` on the relevant account type
//! 3. `use anchor_spl::token;` in the user's program

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
