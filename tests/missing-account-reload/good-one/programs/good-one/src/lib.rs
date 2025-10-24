use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;

mod context;
mod helpers;
mod state;

use context::*;
use helpers::*;
use state::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod good_one {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize()
    }

    pub fn transfer_with_impl_reload(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        // Calls impl method that properly reloads
        ctx.accounts.transfer_with_reload(amount)
    }

    pub fn transfer_with_helper(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        // Use helper function for CPI
        do_transfer(&ctx.accounts.user_account, &ctx.accounts.authority, amount)?;

        // GOOD: Reload after helper CPI
        ctx.accounts.user_account.reload()?;
        let balance = ctx.accounts.user_account.balance;
        msg!("Balance after helper transfer: {}", balance);

        Ok(())
    }

    pub fn transfer_direct_with_reload(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        // Direct CPI
        anchor_lang::solana_program::program::invoke(
            &system_instruction::transfer(
                &ctx.accounts.user_account.key(),
                &ctx.accounts.authority.key(),
                amount,
            ),
            &[
                ctx.accounts.user_account.to_account_info(),
                ctx.accounts.authority.to_account_info(),
            ],
        )?;

        // GOOD: Reload before access
        ctx.accounts.user_account.reload()?;
        let balance = ctx.accounts.user_account.balance;
        msg!("Balance: {}", balance);

        Ok(())
    }

    pub fn no_cpi_no_reload(ctx: Context<Transfer>) -> Result<()> {
        // GOOD: No CPI, so no need to reload
        let balance = ctx.accounts.user_account.balance;
        msg!("Balance: {}", balance);
        Ok(())
    }
}
