use {crate::ErrorCode, anchor_lang::prelude::*};

pub const MAX_LABEL_LEN: usize = 32;
pub const MAX_SIGNERS: usize = 10;

#[account]
#[derive(InitSpace)]
pub struct MultisigConfig {
    pub creator: Pubkey,
    pub threshold: u8,
    pub bump: u8,
    #[max_len(MAX_LABEL_LEN)]
    pub label: String,
    #[max_len(MAX_SIGNERS)]
    pub signers: Vec<Pubkey>,
}

impl MultisigConfig {
    #[inline(always)]
    pub fn initialize(
        &mut self,
        creator: Pubkey,
        threshold: u8,
        bump: u8,
        label: &str,
        signers: &[Pubkey],
    ) -> Result<()> {
        require!(label.len() <= MAX_LABEL_LEN, ErrorCode::LabelTooLong);
        require!(signers.len() <= MAX_SIGNERS, ErrorCode::TooManySigners);

        self.creator = creator;
        self.threshold = threshold;
        self.bump = bump;
        self.label = label.to_owned();
        self.signers = signers.to_vec();

        Ok(())
    }

    #[inline(always)]
    pub fn signers(&self) -> &[Pubkey] {
        &self.signers
    }

    #[inline(always)]
    pub fn set_label(&mut self, creator: &Signer<'_>, label: &str) -> Result<()> {
        require_keys_eq!(self.creator, creator.key(), ErrorCode::UnauthorizedCreator);
        require!(label.len() <= MAX_LABEL_LEN, ErrorCode::LabelTooLong);

        self.label = label.to_owned();
        Ok(())
    }
}
