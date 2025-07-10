// use crate::error::ErrorCode;
use crate::{AccountDeserialize, AccountSerialize, Owner, Result};
use arch_program::{
    bpf_loader::{LoaderState, LoaderStatus, BPF_LOADER_ID},
    program_error::ProgramError,
    pubkey::Pubkey,
};

#[derive(Clone)]
pub struct ProgramData {
    /// Slot is always zero on Arch – kept for API compatibility
    pub slot: u64,
    /// Upgrade authority of the program (if any). `None` once the program
    /// has been finalized.
    pub upgrade_authority_address: Option<Pubkey>,
}

impl AccountDeserialize for ProgramData {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self> {
        Self::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
        // Re-use the LoaderState deserialization we implement below.
        let loader_state = LoaderState::try_deserialize_unchecked(buf)?;

        match loader_state.status {
            LoaderStatus::Retracted | LoaderStatus::Deployed => Ok(ProgramData {
                slot: 0,
                upgrade_authority_address: Some(loader_state.authority_address_or_next_version),
            }),
            LoaderStatus::Finalized => Err(crate::error::ErrorCode::AccountNotProgramData.into()),
        }
    }
}

impl AccountSerialize for ProgramData {
    fn try_serialize<W: std::io::Write>(&self, _writer: &mut W) -> Result<()> {
        // No-op – program data accounts are read-only from the perspective of
        // on-chain programs.
        Ok(())
    }
}

impl Owner for ProgramData {
    fn owner() -> Pubkey {
        BPF_LOADER_ID
    }
}

// -----------------------------------------------------------------------------
// Glue so users can declare `Account<'info, LoaderState>` directly
// -----------------------------------------------------------------------------

impl Owner for LoaderState {
    fn owner() -> Pubkey {
        BPF_LOADER_ID
    }
}

impl AccountSerialize for LoaderState {
    fn try_serialize<W: std::io::Write>(&self, _writer: &mut W) -> Result<()> {
        Ok(())
    }
}

impl AccountDeserialize for LoaderState {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self> {
        Self::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
        // Manual unpacking because `LoaderState` does not implement `serde::Deserialize`.
        const LEN: usize = 32 + 8; // Pubkey (32) + u64 (status)
        if buf.len() < LEN {
            return Err(ProgramError::InvalidAccountData.into());
        }

        // Split and advance the slice so downstream code sees only the
        // remaining account data – this matches the behaviour of other
        // `AccountDeserialize` impls in Anchor.
        let (data, rest) = buf.split_at(LEN);
        *buf = rest;

        // Safety: length checked above.
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&data[..32]);
        let authority_address_or_next_version = Pubkey(key_bytes);

        let mut status_bytes = [0u8; 8];
        status_bytes.copy_from_slice(&data[32..40]);
        let status_val = u64::from_le_bytes(status_bytes);
        let status = match status_val {
            0 => LoaderStatus::Retracted,
            1 => LoaderStatus::Deployed,
            2 => LoaderStatus::Finalized,
            _ => return Err(ProgramError::InvalidAccountData.into()),
        };

        Ok(LoaderState {
            authority_address_or_next_version,
            status,
        })
    }
}

#[cfg(feature = "idl-build")]
mod idl_build {
    use super::*;

    impl crate::IdlBuild for ProgramData {}
    impl crate::Discriminator for ProgramData {
        const DISCRIMINATOR: &'static [u8] = &[];
    }
}
