// use solana_rbpf::declare_builtin_function;

use crate::pubkey::Pubkey;

/// This is native loader
/// used for invoking native programs, this doesn't have a an account on it's own,
/// but native programs use this address in their owner's field.

pub const BPF_LOADER_ID: Pubkey = Pubkey(*b"BpfLoader11111111111111111111111");

pub fn check_id(id: &Pubkey) -> bool {
    id == &BPF_LOADER_ID
}

#[repr(u64)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LoaderStatus {
    /// Program is in maintenance
    Retracted,
    /// Program is ready to be executed
    Deployed,
    /// Same as `Deployed`, but can not be retracted anymore
    Finalized,
}

/// LoaderV4 account states
#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct LoaderState {
    /// Address of signer which can send program management instructions.
    /// Otherwise a forwarding to the next version of the finalized program.
    pub authority_address_or_next_version: Pubkey,
    /// Deployment status.
    pub status: LoaderStatus,
}

impl LoaderState {
    /// Size of a serialized program account.
    pub const fn program_data_offset() -> usize {
        std::mem::size_of::<Self>()
    }
}
