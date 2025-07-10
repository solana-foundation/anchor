//! Instructions for the v4 built-in loader program.

use crate::{
    account::AccountMeta, bpf_loader::BPF_LOADER_ID, instruction::Instruction, pubkey::Pubkey,
};

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum LoaderInstruction {
    /// Write ELF data into an undeployed program account.
    ///
    /// # Account references
    ///   0. `[writable]` The program account to write to.
    ///   1. `[signer]` The authority of the program.
    Write {
        /// Offset at which to write the given bytes.
        offset: u32,
        /// Serialized program data
        #[serde(with = "serde_bytes")]
        bytes: Vec<u8>,
    },

    /// Changes the size of an undeployed program account.
    ///
    /// A program account is automatically initialized when its size is first increased.
    /// In this initial truncate, the program account needs to be a signer and
    /// it also sets the authority needed for subsequent operations.
    /// Decreasing to size zero closes the program account and resets it
    /// into an uninitialized state.
    /// Providing additional lamports upfront might be necessary to reach rent exemption.
    /// Superflous funds are transferred to the recipient account.
    ///
    /// # Account references
    ///   0. `[(signer), writable]` The program account to change the size of.
    ///   1. `[signer]` The authority of the program.
    ///   2. `[writable]` Optional, the recipient account.
    Truncate {
        /// The new size after the operation.
        new_size: u32,
    },

    /// Verify the data of a program account to be a valid ELF.
    ///
    /// If this succeeds the program becomes executable, and is ready to use.
    /// A source program account can be provided to overwrite the data before deployment
    /// in one step, instead retracting the program and writing to it and redeploying it.
    /// The source program is truncated to zero (thus closed) and lamports necessary for
    /// rent exemption are transferred, in case that the source was bigger than the program.
    ///
    /// # Account references
    ///   0. `[writable]` The program account to deploy.
    ///   1. `[signer]` The authority of the program.
    ///   2. `[writable]` Optional, an undeployed source program account to take data and lamports from.
    Deploy,

    /// Undo the deployment of a program account.
    ///
    /// The program is no longer executable and goes into maintenance.
    /// Necessary for writing data and truncating.
    ///
    /// # Account references
    ///   0. `[writable]` The program account to retract.
    ///   1. `[signer]` The authority of the program.
    Retract,

    /// Transfers the authority over a program account.
    ///
    /// # Account references
    ///   0. `[writable]` The program account to change the authority of.
    ///   1. `[signer]` The current authority of the program.
    ///   2. `[signer]` The new authority of the program.
    TransferAuthority,

    /// Finalizes the program account, rendering it immutable.
    ///
    /// # Account references
    ///   0. `[writable]` The program account to change the authority of.
    ///   1. `[signer]` The current authority of the program.
    ///   2. `[]` The next version of the program (can be itself).
    Finalize,
}

pub fn write(
    program_account: Pubkey,
    authority: Pubkey,
    offset: u32,
    bytes: Vec<u8>,
) -> Instruction {
    Instruction {
        program_id: BPF_LOADER_ID,
        accounts: vec![
            AccountMeta::new(program_account, false),
            AccountMeta::new_readonly(authority, true),
        ],
        data: bincode::serialize(&LoaderInstruction::Write { offset, bytes }).unwrap(),
    }
}

pub fn truncate(program_account: Pubkey, authority: Pubkey, new_size: u32) -> Instruction {
    Instruction {
        program_id: BPF_LOADER_ID,
        accounts: vec![
            AccountMeta::new(program_account, true),
            AccountMeta::new_readonly(authority, true),
        ],
        data: bincode::serialize(&LoaderInstruction::Truncate { new_size }).unwrap(),
    }
}

pub fn deploy(program_account: Pubkey, authority: Pubkey) -> Instruction {
    Instruction {
        program_id: BPF_LOADER_ID,
        accounts: vec![
            AccountMeta::new(program_account, false),
            AccountMeta::new_readonly(authority, true),
        ],
        data: bincode::serialize(&LoaderInstruction::Deploy).unwrap(),
    }
}

pub fn retract(program_account: Pubkey, authority: Pubkey) -> Instruction {
    Instruction {
        program_id: BPF_LOADER_ID,
        accounts: vec![
            AccountMeta::new(program_account, false),
            AccountMeta::new_readonly(authority, true),
        ],
        data: bincode::serialize(&LoaderInstruction::Retract).unwrap(),
    }
}

pub fn transfer_authority(
    program_account: Pubkey,
    current_authority: Pubkey,
    new_authority: Pubkey,
) -> Instruction {
    Instruction {
        program_id: BPF_LOADER_ID,
        accounts: vec![
            AccountMeta::new(program_account, true),
            AccountMeta::new_readonly(current_authority, true),
            AccountMeta::new_readonly(new_authority, true),
        ],
        data: bincode::serialize(&LoaderInstruction::TransferAuthority).unwrap(),
    }
}

pub fn finalize(program_account: Pubkey, authority: Pubkey, next_version: Pubkey) -> Instruction {
    Instruction {
        program_id: BPF_LOADER_ID,
        accounts: vec![
            AccountMeta::new(program_account, true),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(next_version, false),
        ],
        data: bincode::serialize(&LoaderInstruction::Finalize).unwrap(),
    }
}
