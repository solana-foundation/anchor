use crate::pubkey::Pubkey;

pub const STAKE_PROGRAM_ID: Pubkey = Pubkey(*b"StakeProgram11111111111111111111");

pub fn check_id(id: &Pubkey) -> bool {
    id == &STAKE_PROGRAM_ID
}
