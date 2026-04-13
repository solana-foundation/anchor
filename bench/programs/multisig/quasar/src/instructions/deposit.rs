use {crate::state::MultisigConfig, quasar_lang::prelude::*};

#[derive(Accounts)]
pub struct Deposit {
    #[account(mut)]
    pub depositor: Signer,
    pub config: Account<MultisigConfig>,
    #[account(mut, seeds = [b"vault", config], bump)]
    pub vault: UncheckedAccount,
    pub system_program: Program<System>,
}

impl Deposit {
    #[inline(always)]
    pub fn deposit(&self, amount: u64) -> Result<(), ProgramError> {
        self.system_program
            .transfer(&self.depositor, &self.vault, amount)
            .invoke()
    }
}
