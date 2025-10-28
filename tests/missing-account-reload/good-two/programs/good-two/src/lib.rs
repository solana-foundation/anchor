use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod good_two {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let user_account = &mut ctx.accounts.user_account;
        user_account.balance = 2000;
        user_account.authority = ctx.accounts.authority.key();
        Ok(())
    }

    pub fn transfer_with_direct_cpi_good(ctx: Context<Transfer>, amount: u64) -> Result<()> {
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

        // GOOD: Reload account after CPI
        ctx.accounts.user_account.reload()?;

        // Now safe to access account data
        let balance = ctx.accounts.user_account.balance;
        msg!("Balance: {}", balance);

        Ok(())
    }

    pub fn transfer_with_macro_cpi_bad(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        // CPI using builder pattern
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.key(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.user_account.to_account_info(),
                    to: ctx.accounts.beneficiary.to_account_info(),
                },
            ),
            amount,
        )?;

        // GOOD: Reload account after CPI
        ctx.accounts.user_account.reload()?;

        // Now safe to access account data
        let balance = ctx.accounts.user_account.balance;
        msg!("Balance: {}", balance);

        Ok(())
    }

    pub fn metadata_access_only(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        // CPI call
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

        // GOOD: Only accessing metadata, not account data
        let key = ctx.accounts.user_account.key();
        let info = ctx.accounts.user_account.to_account_info();
        msg!("Account key: {}, Lamports: {}", key, info.lamports());

        Ok(())
    }

    pub fn no_cpi_at_all(ctx: Context<Transfer>) -> Result<()> {
        // GOOD: No CPI, so no need to reload
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
    /// CHECK: Beneficiary account is only used for receiving SOL
    pub beneficiary: AccountInfo<'info>,
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
