use {
    crate::state::MultisigConfig,
    quasar_lang::{prelude::*, remaining::RemainingAccounts},
};

#[derive(Accounts)]
pub struct Create<'info> {
    pub creator: &'info mut Signer,
    #[account(init, payer = creator, seeds = MultisigConfig::seeds(creator), bump)]
    pub config: Account<MultisigConfig<'info>>,
    pub rent: &'info Sysvar<Rent>,
    pub system_program: &'info Program<System>,
}

impl<'info> Create<'info> {
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
            *self.creator.address(),
            threshold,
            bumps.config,
            "",
            signers,
            self.creator.to_account_view(),
            Some(&**self.rent),
        )
    }
}
