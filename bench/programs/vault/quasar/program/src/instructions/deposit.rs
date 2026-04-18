use quasar_lang::prelude::*;

#[derive(Accounts)]
pub struct Deposit {
    #[account(mut)]
    pub user: Signer,
    #[account(mut, seeds = [b"vault", user], bump)]
    pub vault: UncheckedAccount,
    pub system_program: Program<System>,
}

impl Deposit {
    #[inline(always)]
    pub fn deposit(&self, amount: u64) -> Result<(), ProgramError> {
        self.system_program
            .transfer(&self.user, &self.vault, amount)
            .invoke()
    }
}
