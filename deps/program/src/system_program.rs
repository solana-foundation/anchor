use crate::pubkey::Pubkey;

pub const SYSTEM_PROGRAM_ID: Pubkey = Pubkey::system_program();

pub fn check_id(id: &Pubkey) -> bool {
    id == &SYSTEM_PROGRAM_ID
}
