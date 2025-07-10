use crate::pubkey::Pubkey;

pub const VOTE_PROGRAM_ID: Pubkey = Pubkey(*b"VoteProgram111111111111111111111");

pub fn check_id(id: &Pubkey) -> bool {
    id == &VOTE_PROGRAM_ID
}
