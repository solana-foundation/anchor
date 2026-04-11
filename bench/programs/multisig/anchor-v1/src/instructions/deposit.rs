use {
    crate::state::MultisigConfig,
    anchor_lang::{prelude::*, system_program},
};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,
    pub config: Account<'info, MultisigConfig>,
    #[account(mut, seeds = [b"vault", config.key().as_ref()], bump)]
    pub vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    #[inline(always)]
    pub fn deposit(&self, amount: u64) -> Result<()> {
        system_program::transfer(
            CpiContext::new(
                self.system_program.key(),
                system_program::Transfer {
                    from: self.depositor.to_account_info(),
                    to: self.vault.to_account_info(),
                },
            ),
            amount,
        )
    }
}
