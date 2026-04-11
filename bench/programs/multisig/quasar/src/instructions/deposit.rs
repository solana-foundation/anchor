use {crate::state::MultisigConfig, quasar_lang::prelude::*};

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub depositor: &'info mut Signer,
    pub config: Account<MultisigConfig<'info>>,
    #[account(mut, seeds = [b"vault", config], bump)]
    pub vault: &'info mut UncheckedAccount,
    pub system_program: &'info Program<System>,
}

impl<'info> Deposit<'info> {
    #[inline(always)]
    pub fn deposit(&self, amount: u64) -> Result<(), ProgramError> {
        self.system_program
            .transfer(self.depositor, self.vault, amount)
            .invoke()
    }
}
