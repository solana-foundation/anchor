use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod double_cpi_test {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let user_account = &mut ctx.accounts.user_account;
        user_account.balance = 1000;
        user_account.authority = ctx.accounts.authority.key();
        Ok(())
    }

    // BAD: Second CPI invalidates the previous reload
    pub fn double_cpi_without_second_reload(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        // First CPI
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

        // Reload after first CPI - this is good
        ctx.accounts.user_account.reload()?;

        // Second CPI - this invalidates the previous reload!
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

        // BAD: Accessing data after second CPI without reload
        let balance = ctx.accounts.user_account.balance;
        msg!("Balance: {}", balance);

        Ok(())
    }

    // GOOD: Reload after each CPI
    pub fn double_cpi_with_both_reloads(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        // First CPI
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

        // Reload after first CPI
        ctx.accounts.user_account.reload()?;

        // Second CPI
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

        // GOOD: Reload after second CPI before accessing data
        ctx.accounts.user_account.reload()?;

        // Now safe to access account data
        let balance = ctx.accounts.user_account.balance;
        msg!("Balance: {}", balance);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + UserAccount::INIT_SPACE
    )]
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Transfer<'info> {
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct UserAccount {
    pub balance: u64,
    pub authority: Pubkey,
}

impl Space for UserAccount {
    const INIT_SPACE: usize = 8 + 8 + 32;
}
