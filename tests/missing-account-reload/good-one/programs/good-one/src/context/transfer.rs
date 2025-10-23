use crate::state::*;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;

#[derive(Accounts)]
pub struct Transfer<'info> {
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Transfer<'info> {
    pub fn transfer_with_reload(&mut self, amount: u64) -> Result<()> {
        anchor_lang::solana_program::program::invoke(
            &system_instruction::transfer(&self.user_account.key(), &self.authority.key(), amount),
            &[
                self.user_account.to_account_info(),
                self.authority.to_account_info(),
            ],
        )?;

        // GOOD: Reload after CPI
        self.user_account.reload()?;
        let balance = self.user_account.balance;
        msg!("Balance: {}", balance);

        Ok(())
    }
}
