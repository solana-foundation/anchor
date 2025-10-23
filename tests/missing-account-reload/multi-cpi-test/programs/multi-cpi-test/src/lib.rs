use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod multi_cpi_test {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let user_account = &mut ctx.accounts.user_account;
        user_account.balance = 1000;
        user_account.authority = ctx.accounts.authority.key();
        Ok(())
    }

    // BAD: Multiple CPIs with reload only after first one
    pub fn multi_cpi_missing_last_reload(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        // CPI 1
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

        ctx.accounts.user_account.reload()?;

        // CPI 2
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

        ctx.accounts.user_account.reload()?;

        // CPI 3
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

        ctx.accounts.user_account.reload()?;

        // CPI 4
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

        ctx.accounts.user_account.reload()?;

        // CPI 5 - Fifth CPI!
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

        // BAD: Missing reload after 5th CPI!
        let balance = ctx.accounts.user_account.balance;
        msg!("Balance: {}", balance);

        Ok(())
    }

    // GOOD: Reload after every CPI including the last one
    pub fn multi_cpi_with_all_reloads(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        // CPI 1
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
        ctx.accounts.user_account.reload()?;

        // CPI 2
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
        ctx.accounts.user_account.reload()?;

        // CPI 3
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
        ctx.accounts.user_account.reload()?;

        // CPI 4
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
        ctx.accounts.user_account.reload()?;

        // CPI 5
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

        // GOOD: Reload after 5th CPI before accessing data
        ctx.accounts.user_account.reload()?;

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
