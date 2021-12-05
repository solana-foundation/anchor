use crate::{AccountDeserialize, AccountSerialize, Owner};
use solana_program::{
    bpf_loader_upgradeable::UpgradeableLoaderState, program_error::ProgramError, pubkey::Pubkey,
};

#[derive(Clone)]
pub struct ProgramData {
    pub slot: u64,
    pub upgrade_authority_address: Option<Pubkey>,
}

impl AccountDeserialize for ProgramData {
    fn try_deserialize(
        buf: &mut &[u8],
    ) -> Result<Self, solana_program::program_error::ProgramError> {
        ProgramData::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(
        buf: &mut &[u8],
    ) -> Result<Self, solana_program::program_error::ProgramError> {
        let program_state = AccountDeserialize::try_deserialize_unchecked(buf)?;

        match program_state {
            UpgradeableLoaderState::Uninitialized => {
                Err(anchor_lang::error::ErrorCode::AccountNotProgramData.into())
            }
            UpgradeableLoaderState::Buffer {
                authority_address: _,
            } => Err(anchor_lang::error::ErrorCode::AccountNotProgramData.into()),
            UpgradeableLoaderState::Program {
                programdata_address: _,
            } => Err(anchor_lang::error::ErrorCode::AccountNotProgramData.into()),
            UpgradeableLoaderState::ProgramData {
                slot,
                upgrade_authority_address,
            } => Ok(ProgramData {
                slot,
                upgrade_authority_address,
            }),
        }
    }
}

impl AccountSerialize for ProgramData {
    fn try_serialize<W: std::io::Write>(
        &self,
        _writer: &mut W,
    ) -> Result<(), solana_program::program_error::ProgramError> {
        // no-op
        Ok(())
    }
}

impl Owner for ProgramData {
    fn owner() -> solana_program::pubkey::Pubkey {
        anchor_lang::solana_program::bpf_loader_upgradeable::ID
    }
}

impl Owner for UpgradeableLoaderState {
    fn owner() -> Pubkey {
        anchor_lang::solana_program::bpf_loader_upgradeable::ID
    }
}

impl AccountSerialize for UpgradeableLoaderState {
    fn try_serialize<W: std::io::Write>(&self, _writer: &mut W) -> Result<(), ProgramError> {
        // no-op
        Ok(())
    }
}

impl AccountDeserialize for UpgradeableLoaderState {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        UpgradeableLoaderState::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        bincode::deserialize(buf).map_err(|_| ProgramError::InvalidAccountData)
    }
}
