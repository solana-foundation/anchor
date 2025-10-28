use crate::state::*;
use anchor_lang::prelude::*;
use crate::context::utils::transfer;

#[derive(Accounts)]
pub struct Transfer<'info> {
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Transfer<'info> {
    pub fn transfer(&mut self, amount: u64) -> Result<()> {
        transfer(&mut self.user_account, &mut self.authority, amount)?;
        // BAD: Missing reload() after CPI!
        let balance = self.user_account.balance;
        msg!("Balance: {}", balance);
        Ok(())
    }
}
