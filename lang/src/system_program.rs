use crate::*;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub use solana_program::system_program::ID;

#[derive(Debug, Clone)]
pub struct System;

impl anchor_lang::AccountDeserialize for System {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        System::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(_buf: &mut &[u8]) -> Result<Self, ProgramError> {
        Ok(System)
    }
}

impl anchor_lang::Id for System {
    fn id() -> Pubkey {
        ID
    }
}
