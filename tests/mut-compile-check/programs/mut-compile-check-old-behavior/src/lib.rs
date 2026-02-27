//! Demonstrates the OLD/BUGGY behavior with Account<>.
//!
//! This program COMPILES but the mutation DOES NOT PERSIST because
//! #[account(mut)] is missing. This is the silent failure bug that
//! ReadOnlyAccount is designed to prevent.
//!
//! Users should use ReadOnlyAccount instead of Account for non-mutable accounts
//! to get compile-time safety.
use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[account]
pub struct MyData {
    pub value: u64,
}

/// Accounts struct without #[account(mut)] - using Account (old behavior)
/// This demonstrates the bug: mutation appears to work but doesn't persist
#[derive(Accounts)]
pub struct OldBehavior<'info> {
    // WARNING: No #[account(mut)] but using Account<> allows mutation
    // The mutation will compile but NOT persist - this is the bug!
    pub data: Account<'info, MyData>,
}

#[program]
pub mod mut_compile_check_old_behavior {
    use super::*;

    /// WARNING: This compiles but mutation doesn't persist!
    ///
    /// This demonstrates the bug that ReadOnlyAccount is designed to prevent.
    /// With Account<>, you can mutate without #[account(mut)], but the changes
    /// won't be saved because exit() doesn't serialize non-mutable accounts.
    ///
    /// Solution: Use ReadOnlyAccount<> for non-mutable accounts to get
    /// compile-time errors instead of silent runtime failures.
    pub fn mutate_without_mut_old_behavior(
        ctx: Context<OldBehavior>,
        new_value: u64,
    ) -> Result<()> {
        // This compiles but the change is silently discarded!
        ctx.accounts.data.value = new_value;
        Ok(())
    }
}
