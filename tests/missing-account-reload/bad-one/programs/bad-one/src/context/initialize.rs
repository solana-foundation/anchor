use anchor_lang::prelude::*;
use crate::state::*;

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


impl<'info> Initialize<'info> {
    pub fn initialize(&mut self) -> Result<()> {
        self.user_account.balance = 1000;
        self.user_account.authority = self.authority.key();
        Ok(())
    }
}