use {
    crate::state::MultisigConfig,
    quasar_lang::{prelude::*, remaining::RemainingAccounts},
};

#[derive(Accounts)]
pub struct ExecuteTransfer<'info> {
    #[account(
        has_one = creator,
        seeds = MultisigConfig::seeds(creator),
        bump = config.bump
    )]
    pub config: Account<MultisigConfig<'info>>,
    pub creator: &'info UncheckedAccount,
    #[account(mut, seeds = [b"vault", config], bump)]
    pub vault: &'info mut UncheckedAccount,
    pub recipient: &'info mut UncheckedAccount,
    pub system_program: &'info Program<System>,
}

impl<'info> ExecuteTransfer<'info> {
    #[inline(always)]
    pub fn verify_and_transfer(
        &self,
        amount: u64,
        bumps: &ExecuteTransferBumps,
        remaining: RemainingAccounts,
    ) -> Result<(), ProgramError> {
        let stored_signers = self.config.signers();
        let threshold = self.config.threshold;

        let mut approvals = 0u32;
        for account in remaining.iter() {
            let account = account?;
            if !account.is_signer() {
                continue;
            }
            let addr = account.address();
            for stored in stored_signers {
                if addr == stored {
                    approvals = approvals.wrapping_add(1);
                    break;
                }
            }
        }

        if approvals < threshold as u32 {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let seeds = self.vault_seeds(bumps);
        self.system_program
            .transfer(self.vault, self.recipient, amount)
            .invoke_signed(&seeds)?;
        Ok(())
    }
}
