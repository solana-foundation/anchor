use anchor_lang_v2::prelude::*;

pub const MAX_LABEL_LEN: usize = 32;
pub const MAX_SIGNERS: usize = 10;

/// Zerocopy multisig config — length-prefixed `PodVec` fields for label and
/// signers, matching quasar's "dynamic spirit" while keeping fixed storage
/// offsets (each `PodVec` reserves its max capacity inline).
///
/// Layout (bytes):
///   8    discriminator
///   32   creator: Address
///   1    threshold
///   1    bump
///   2+32 label: PodVec<u8, 32>         (u16 len + 32 bytes)
///   2+320 signers: PodVec<Address, 10> (u16 len + 10*32 bytes)
#[account]
pub struct MultisigConfig {
    pub creator: Address,
    pub threshold: u8,
    pub bump: u8,
    pub label: PodVec<u8, MAX_LABEL_LEN>,
    pub signers: PodVec<Address, MAX_SIGNERS>,
}

impl MultisigConfig {
    pub fn label_str(&self) -> &str {
        // SAFETY: label is validated as UTF-8 during set_label/initialize.
        unsafe { core::str::from_utf8_unchecked(self.label.as_slice()) }
    }

    pub fn active_signers(&self) -> &[Address] {
        self.signers.as_slice()
    }
}
