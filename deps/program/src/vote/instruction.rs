use crate::{
    account::AccountMeta,
    decode_error::DecodeError,
    instruction::{Instruction, InstructionError},
    pubkey::Pubkey,
    system_instruction,
    vote::state::VoteState,
};
use num_derive::{FromPrimitive, ToPrimitive};
use thiserror::Error;

use super::{program::VOTE_PROGRAM_ID, state::VoteInit};

#[derive(Error, Debug, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum VoteError {
    #[error("not enough credits to redeem")]
    NoCreditsToRedeem,
    #[error("too soon to reauthorize")]
    TooSoonToReauthorize,
}

impl From<VoteError> for InstructionError {
    fn from(error: VoteError) -> Self {
        InstructionError::VoteError(error)
    }
}

impl<E> DecodeError<E> for VoteError {
    fn type_of() -> &'static str {
        "VoteError"
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum VoteInstruction {
    /// Initialize a stake with lockup and authorization information
    ///
    /// # Account references
    ///   0. `[WRITE]` Uninitialized validator account
    ///   1. `[]` Rent sysvar
    ///
    /// Authorized carries pubkeys that must sign staker transactions
    ///   and withdrawer transactions.
    /// Lockup carries information about withdrawal restrictions
    Initialize(VoteInit),
    Authorize(Pubkey),
    UpdateCommission(u8),
    InitializeSharedValidatorAccount(Pubkey, Vec<u8>, Vec<Pubkey>),
    UpdatePubkeyPackage(Vec<u8>),
    AddPeerToWhitelist(Pubkey),
    RemovePeerFromWhitelist(Pubkey),
}

pub fn initialize(vote_pubkey: &Pubkey, vote_init: &VoteInit) -> Instruction {
    Instruction::new_with_bincode(
        VOTE_PROGRAM_ID,
        &VoteInstruction::Initialize(*vote_init),
        vec![AccountMeta::new(*vote_pubkey, false)],
    )
}

pub fn create_account(
    from_pubkey: &Pubkey,
    vote_pubkey: &Pubkey,
    vote_init: &VoteInit,
    lamports: u64,
) -> Vec<Instruction> {
    vec![
        system_instruction::create_account(
            from_pubkey,
            vote_pubkey,
            lamports,
            VoteState::size_of_new() as u64,
            &VOTE_PROGRAM_ID,
        ),
        initialize(vote_pubkey, vote_init),
    ]
}

pub fn authorize(vote_pubkey: &Pubkey, authority: &Pubkey, new_authority: &Pubkey) -> Instruction {
    Instruction::new_with_bincode(
        VOTE_PROGRAM_ID,
        &VoteInstruction::Authorize(*new_authority),
        vec![
            AccountMeta::new(*vote_pubkey, false),
            AccountMeta::new(*authority, true),
        ],
    )
}

pub fn update_commission(vote_pubkey: &Pubkey, authority: &Pubkey, commission: u8) -> Instruction {
    Instruction::new_with_bincode(
        VOTE_PROGRAM_ID,
        &VoteInstruction::UpdateCommission(commission),
        vec![
            AccountMeta::new(*vote_pubkey, false),
            AccountMeta::new(*authority, true),
        ],
    )
}

pub fn initialize_shared_validator_account(
    shared_validator_pubkey: &Pubkey,
    bootnode_pubkey: &Pubkey,
    serialized_pubkey_package: &Vec<u8>,
    whitelist: &Vec<Pubkey>,
) -> Instruction {
    Instruction::new_with_bincode(
        VOTE_PROGRAM_ID,
        &VoteInstruction::InitializeSharedValidatorAccount(
            *bootnode_pubkey,
            serialized_pubkey_package.clone(),
            whitelist.clone(),
        ),
        vec![AccountMeta::new(*shared_validator_pubkey, false)],
    )
}

pub fn update_pubkey_package(
    shared_validator_pubkey: &Pubkey,
    serialized_pubkey_package: &Vec<u8>,
) -> Instruction {
    Instruction::new_with_bincode(
        VOTE_PROGRAM_ID,
        &VoteInstruction::UpdatePubkeyPackage(serialized_pubkey_package.clone()),
        vec![AccountMeta::new(*shared_validator_pubkey, false)],
    )
}

pub fn add_peer_to_whitelist(
    shared_validator_pubkey: &Pubkey,
    bootnode_pubkey: &Pubkey,
    peer_pubkey: Pubkey,
) -> Instruction {
    Instruction::new_with_bincode(
        VOTE_PROGRAM_ID,
        &VoteInstruction::AddPeerToWhitelist(peer_pubkey),
        vec![
            AccountMeta::new(*shared_validator_pubkey, false),
            AccountMeta::new(*bootnode_pubkey, true),
        ],
    )
}

pub fn remove_peer_from_whitelist(
    shared_validator_pubkey: &Pubkey,
    bootnode_pubkey: &Pubkey,
    peer_pubkey: Pubkey,
) -> Instruction {
    Instruction::new_with_bincode(
        VOTE_PROGRAM_ID,
        &VoteInstruction::RemovePeerFromWhitelist(peer_pubkey),
        vec![
            AccountMeta::new(*shared_validator_pubkey, false),
            AccountMeta::new(*bootnode_pubkey, true),
        ],
    )
}
