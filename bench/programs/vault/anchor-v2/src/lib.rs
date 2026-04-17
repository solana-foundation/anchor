//! Anchor v2 port of the quasar vault benchmark.
//!
//! Two instructions:
//!   - `deposit(amount)` — transfers SOL from `user` to `vault` PDA via CPI
//!   - `withdraw(amount)` — direct lamport arithmetic (vault has no allocated
//!     data so the runtime's write-ownership check permits it)
//!
//! Matches the shape of `quasar-vault` in `examples/vault` so the cross-
//! framework comparison is apples-to-apples.

use anchor_lang_v2::prelude::*;

declare_id!("33333333333333333333333333333333333333333333");

#[program]
pub mod vault_v2 {
    use super::*;

    #[discrim = 0]
    pub fn deposit(ctx: &mut Context<Deposit>, amount: u64) -> Result<()> {
        pinocchio_system::instructions::Transfer {
            from: ctx.accounts.user.account(),
            to: ctx.accounts.vault.account(),
            lamports: amount,
        }
        .invoke()?;
        Ok(())
    }

    #[discrim = 1]
    pub fn withdraw(ctx: &mut Context<Withdraw>, amount: u64) -> Result<()> {
        // AccountView is Copy — copying gives an owned value we can call
        // `set_lamports(&mut self)` on. The write goes through a raw
        // pointer into the account's serialized data buffer, so both the
        // original and the copy point to the same backing memory.
        let mut vault = *ctx.accounts.vault.account();
        let mut user = *ctx.accounts.user.account();
        vault.set_lamports(vault.lamports() - amount);
        user.set_lamports(user.lamports() + amount);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Deposit {
    #[account(mut)]
    pub user: Signer,
    #[account(mut, seeds = [b"vault", user.address().as_ref()], bump)]
    pub vault: UncheckedAccount,
    pub system_program: Program<System>,
}

#[derive(Accounts)]
pub struct Withdraw {
    #[account(mut)]
    pub user: Signer,
    #[account(mut, seeds = [b"vault", user.address().as_ref()], bump)]
    pub vault: UncheckedAccount,
}
