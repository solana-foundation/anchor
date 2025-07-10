use crate::pubkey::Pubkey;

/// This is native loader
/// used for invoking native programs, this doesn't have a an account on it's own,
/// but native programs use this address in their owner's field.

pub const NATIVE_LOADER_ID: Pubkey = Pubkey(*b"NativeLoader11111111111111111111");

pub fn check_id(id: &Pubkey) -> bool {
    id == &NATIVE_LOADER_ID
}
