//! Clock module provides time-related functionality for the program.
use borsh::{BorshDeserialize, BorshSerialize};

// Add the Clock struct definition
#[derive(Debug, Clone, Copy, Default, BorshSerialize, BorshDeserialize)]
pub struct Clock {
    pub slot: u64,
    pub epoch: u64,
    pub unix_timestamp: i64,
}

// -----------------------------------------------------------------------------
// Sysvar implementation
// -----------------------------------------------------------------------------

use crate::{account::AccountInfo, program_error::ProgramError};

impl crate::sysvar::Sysvar for Clock {
    fn from_account_info(account_info: &AccountInfo) -> Result<Self, ProgramError> {
        // Deserialize via Borsh from the account's data field. Anchor and Solana both
        // expect sysvars to live in the account data section.
        use borsh::BorshDeserialize;

        let data_ref = account_info.try_borrow_data()?;
        // Convert Ref<&mut [u8]> to &[u8] for deserialization.  We do not need to
        // mutate the slice, so immutable access is sufficient.
        Clock::try_from_slice(&data_ref[..]).map_err(|_| ProgramError::InvalidAccountData)
    }

    fn get() -> Result<Self, ProgramError> {
        // Ask the Arch VM for the current clock via syscall.
        let mut clock = Clock::default();
        // Safety: the syscall expects a valid pointer to a `Clock` instance. We
        // allocated `clock` on the stack above, so the pointer is safe for the
        // VM to write into.
        let result = unsafe {
            crate::syscalls::arch_get_clock(&mut clock as *mut Clock)
        };

        if result == 0 {
            Ok(clock)
        } else {
            // Currently we treat *any* non-zero return value as an invalid
            // argument.  This can be refined once the Arch VM exposes a stable
            // error ABI.
            Err(ProgramError::InvalidArgument)
        }
    }
}
