use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("33333333333333333333333333333333333333333333");

#[program]
pub mod vault_v1 {
    use super::*;

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        let cpi_ctx = CpiContext::new(
            *ctx.accounts.system_program.key,
            system_program::Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        system_program::transfer(cpi_ctx, amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        // Vault is program-owned and data-less, so we can mutate lamports
        // directly. Matches the v2 + quasar variants.
        let vault = &mut ctx.accounts.vault.to_account_info();
        let user = &mut ctx.accounts.user.to_account_info();
        **vault.try_borrow_mut_lamports()? -= amount;
        **user.try_borrow_mut_lamports()? += amount;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: PDA vault, lamports-only.
    #[account(mut, seeds = [b"vault", user.key.as_ref()], bump)]
    pub vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: PDA vault, lamports-only.
    #[account(mut, seeds = [b"vault", user.key.as_ref()], bump)]
    pub vault: UncheckedAccount<'info>,
}
