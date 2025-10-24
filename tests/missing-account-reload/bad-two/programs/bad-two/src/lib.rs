use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod bad_two {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let user_account = &mut ctx.accounts.user_account;
        user_account.balance = 2000;
        user_account.authority = ctx.accounts.authority.key();
        Ok(())
    }

    pub fn transfer_with_direct_cpi_bad(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        // Direct CPI - should trigger warning
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

        // BAD: Access account without reload after CPI
        let balance = ctx.accounts.user_account.balance;
        msg!("Balance: {}", balance);

        Ok(())
    }

    pub fn transfer_with_impl_method_bad(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        // Call impl method that does CPI
        ctx.accounts.transfer_impl(amount)
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

// Impl block OUTSIDE #[program] module - tests file-level scanning
impl<'info> Transfer<'info> {
    pub fn transfer_impl(&mut self, amount: u64) -> Result<()> {
        // CPI using invoke
        anchor_lang::solana_program::program::invoke(
            &system_instruction::transfer(
                &self.user_account.key(),
                &self.beneficiary.key(),
                amount,
            ),
            &[
                self.user_account.to_account_info(),
                self.beneficiary.to_account_info(),
            ],
        )?;

        // BAD: Access account without reload after CPI in impl method
        let balance = self.user_account.balance;
        msg!("Balance after impl transfer: {}", balance);

        Ok(())
    }
}

#[account]
pub struct UserAccount {
    pub balance: u64,
    pub authority: Pubkey,
}

impl Space for UserAccount {
    const INIT_SPACE: usize = 8 + 8 + 32;
}
