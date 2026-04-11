use {
    crate::{state::MultisigConfig, ErrorCode, MAX_SIGNERS},
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct Create<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(
        init,
        payer = creator,
        space = 8 + MultisigConfig::INIT_SPACE,
        seeds = [b"multisig", creator.key().as_ref()],
        bump
    )]
    pub config: Account<'info, MultisigConfig>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

impl<'info> Create<'info> {
    #[inline(always)]
    pub fn create_multisig(
        &mut self,
        threshold: u8,
        bumps: &CreateBumps,
        remaining: &[AccountInfo<'info>],
    ) -> Result<()> {
        let mut addrs = core::mem::MaybeUninit::<[Pubkey; MAX_SIGNERS]>::uninit();
        let addrs_ptr = addrs.as_mut_ptr() as *mut Pubkey;
        let mut count = 0usize;

        for account in remaining {
            if count >= MAX_SIGNERS {
                return err!(ErrorCode::TooManySigners);
            }
            if !account.is_signer {
                return err!(ErrorCode::MissingRequiredSignature);
            }

            unsafe { core::ptr::write(addrs_ptr.add(count), *account.key) };
            count = count.wrapping_add(1);
        }

        if threshold == 0 || threshold as usize > count {
            return err!(ErrorCode::InvalidThreshold);
        }

        let signers = unsafe { core::slice::from_raw_parts(addrs_ptr, count) };

        self.config
            .initialize(self.creator.key(), threshold, bumps.config, "", signers)
    }
}
