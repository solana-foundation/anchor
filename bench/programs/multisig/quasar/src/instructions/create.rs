use {
    crate::state::{MultisigConfig, MultisigConfigInner},
    quasar_lang::{prelude::*, remaining::RemainingAccounts},
};

#[derive(Accounts)]
pub struct Create {
    #[account(mut)]
    pub creator: Signer,
    #[account(init, payer = creator, seeds = MultisigConfig::seeds(creator), bump)]
    pub config: Account<MultisigConfig>,
    pub rent: Sysvar<Rent>,
    pub system_program: Program<System>,
}

impl Create {
    #[inline(always)]
    pub fn create_multisig(
        &mut self,
        threshold: u8,
        bumps: &CreateBumps,
        remaining: RemainingAccounts,
    ) -> Result<(), ProgramError> {
        let mut addrs = core::mem::MaybeUninit::<[Address; 10]>::uninit();
        let addrs_ptr = addrs.as_mut_ptr() as *mut Address;
        let mut count = 0usize;

        for account in remaining.iter() {
            let account = account?;
            if count >= 10 {
                return Err(ProgramError::InvalidArgument);
            }
            if !account.is_signer() {
                return Err(ProgramError::MissingRequiredSignature);
            }
            // SAFETY: count < 10, so addrs_ptr.add(count) is within the 10-element array.
            unsafe { core::ptr::write(addrs_ptr.add(count), *account.address()) };
            count = count.wrapping_add(1);
        }

        if threshold == 0 || threshold as usize > count {
            return Err(ProgramError::InvalidArgument);
        }

        // SAFETY: Elements 0..count were initialized by the loop above.
        let signers = unsafe { core::slice::from_raw_parts(addrs_ptr, count) };

        self.config.set_inner(
            MultisigConfigInner {
                creator: *self.creator.address(),
                threshold,
                bump: bumps.config,
                label: "",
                signers,
            },
            self.creator.to_account_view(),
            self.rent.lamports_per_byte(),
            self.rent.exemption_threshold_raw(),
        )
    }
}
