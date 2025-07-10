use {
    crate::{
        account::AccountMeta,
        decode_error::DecodeError,
        instruction::{Instruction, InstructionError},
        pubkey::Pubkey,
        stake::{
            program::STAKE_PROGRAM_ID,
            state::{Authorized, StakeAuthorize, StakeState},
        },
        system_instruction,
    },
    num_derive::{FromPrimitive, ToPrimitive},
    serde_derive::{Deserialize, Serialize},
    thiserror::Error,
};

/// Reasons the stake might have had an error
#[derive(Error, Debug, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum StakeError {
    #[error("not enough credits to redeem")]
    NoCreditsToRedeem,

    #[error("lockup has not yet expired")]
    LockupInForce,

    #[error("stake already deactivated")]
    AlreadyDeactivated,

    #[error("one re-delegation permitted per epoch")]
    TooSoonToRedelegate,

    #[error("split amount is more than is staked")]
    InsufficientStake,

    #[error("stake account with transient stake cannot be merged")]
    MergeTransientStake,

    #[error("stake account merge failed due to different authority, lockups or state")]
    MergeMismatch,

    #[error("custodian address not present")]
    CustodianMissing,

    #[error("custodian signature not present")]
    CustodianSignatureMissing,

    #[error("insufficient voting activity in the reference vote account")]
    InsufficientReferenceVotes,

    #[error("stake account is not delegated to the provided vote account")]
    VoteAddressMismatch,

    #[error(
        "stake account has not been delinquent for the minimum epochs required for deactivation"
    )]
    MinimumDelinquentEpochsForDeactivationNotMet,

    #[error("delegation amount is less than the minimum")]
    InsufficientDelegation,

    #[error("stake account with transient or inactive stake cannot be redelegated")]
    RedelegateTransientOrInactiveStake,

    #[error("stake redelegation to the same vote account is not permitted")]
    RedelegateToSameVoteAccount,

    #[error("redelegated stake must be fully activated before deactivation")]
    RedelegatedStakeMustFullyActivateBeforeDeactivationIsPermitted,
}

impl From<StakeError> for InstructionError {
    fn from(error: StakeError) -> Self {
        InstructionError::StakeError(error)
    }
}

impl<E> DecodeError<E> for StakeError {
    fn type_of() -> &'static str {
        "StakeError"
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum StakeInstruction {
    /// Initialize a stake with lockup and authorization information
    ///
    /// # Account references
    ///   0. `[WRITE]` Uninitialized stake account
    ///   1. `[]` Rent sysvar
    ///
    /// Authorized carries pubkeys that must sign staker transactions
    ///   and withdrawer transactions.
    /// Lockup carries information about withdrawal restrictions
    Initialize(Authorized),

    /// Authorize a key to manage stake or withdrawal
    ///
    /// # Account references
    ///   0. `[WRITE]` Stake account to be updated
    ///   1. `[]` Clock sysvar
    ///   2. `[SIGNER]` The stake or withdraw authority
    ///   3. Optional: `[SIGNER]` Lockup authority, if updating StakeAuthorize::Withdrawer before
    ///      lockup expiration
    Authorize(Pubkey, StakeAuthorize),

    /// Delegate a stake to a particular vote account
    ///
    /// # Account references
    ///   0. `[WRITE]` Initialized stake account to be delegated
    ///   1. `[]` Vote account to which this stake will be delegated
    ///   2. `[]` Clock sysvar
    ///   3. `[]` Stake history sysvar that carries stake warmup/cooldown history
    ///   4. `[]` Address of config account that carries stake config
    ///   5. `[SIGNER]` Stake authority
    ///
    /// The entire balance of the staking account is staked.  DelegateStake
    ///   can be called multiple times, but re-delegation is delayed
    ///   by one epoch
    DelegateStake,

    /// Withdraw unstaked lamports from the stake account
    ///
    /// # Account references
    ///   0. `[WRITE]` Stake account from which to withdraw
    ///   1. `[WRITE]` Recipient account
    ///   2. `[]` Clock sysvar
    ///   3. `[]` Stake history sysvar that carries stake warmup/cooldown history
    ///   4. `[SIGNER]` Withdraw authority
    ///   5. Optional: `[SIGNER]` Lockup authority, if before lockup expiration
    ///
    /// The u64 is the portion of the stake account balance to be withdrawn,
    ///    must be `<= StakeAccount.lamports - staked_lamports`.
    Withdraw(u64),

    /// Deactivates the stake in the account
    ///
    /// # Account references
    ///   0. `[WRITE]` Delegated stake account
    ///   1. `[]` Clock sysvar
    ///   2. `[SIGNER]` Stake authority
    Deactivate,
}

pub fn initialize(stake_pubkey: &Pubkey, authorized: &Authorized) -> Instruction {
    Instruction::new_with_bincode(
        STAKE_PROGRAM_ID,
        &StakeInstruction::Initialize(*authorized),
        vec![
            AccountMeta::new(*stake_pubkey, false),
            // AccountMeta::new_readonly(sysvar::rent::STAKE_PROGRAM_ID, false),
        ],
    )
}

pub fn create_account(
    from_pubkey: &Pubkey,
    stake_pubkey: &Pubkey,
    authorized: &Authorized,
    // lockup: &Lockup,
    lamports: u64,
) -> Vec<Instruction> {
    vec![
        system_instruction::create_account(
            from_pubkey,
            stake_pubkey,
            lamports,
            StakeState::size_of() as u64,
            &STAKE_PROGRAM_ID,
        ),
        initialize(stake_pubkey, authorized),
    ]
}

// fn _split(
//     stake_pubkey: &Pubkey,
//     authorized_pubkey: &Pubkey,
//     lamports: u64,
//     split_stake_pubkey: &Pubkey,
// ) -> Instruction {
//     let account_metas = vec![
//         AccountMeta::new(*stake_pubkey, false),
//         AccountMeta::new(*split_stake_pubkey, false),
//         AccountMeta::new_readonly(*authorized_pubkey, true),
//     ];

//     Instruction::new_with_bincode(
//         STAKE_PROGRAM_ID,
//         &StakeInstruction::Split(lamports),
//         account_metas,
//     )
// }

// pub fn split(
//     stake_pubkey: &Pubkey,
//     authorized_pubkey: &Pubkey,
//     lamports: u64,
//     split_stake_pubkey: &Pubkey,
// ) -> Vec<Instruction> {
//     vec![
//         system_instruction::allocate(split_stake_pubkey, StakeStateV2::size_of() as u64),
//         system_instruction::assign(split_stake_pubkey, &STAKE_PROGRAM_ID),
//         _split(
//             stake_pubkey,
//             authorized_pubkey,
//             lamports,
//             split_stake_pubkey,
//         ),
//     ]
// }

// pub fn merge(
//     destination_stake_pubkey: &Pubkey,
//     source_stake_pubkey: &Pubkey,
//     authorized_pubkey: &Pubkey,
// ) -> Vec<Instruction> {
//     let account_metas = vec![
//         AccountMeta::new(*destination_stake_pubkey, false),
//         AccountMeta::new(*source_stake_pubkey, false),
//         AccountMeta::new_readonly(*authorized_pubkey, true),
//     ];

//     vec![Instruction::new_with_bincode(
//         STAKE_PROGRAM_ID,
//         &StakeInstruction::Merge,
//         account_metas,
//     )]
// }

pub fn create_account_and_delegate_stake(
    from_pubkey: &Pubkey,
    stake_pubkey: &Pubkey,
    vote_pubkey: &Pubkey,
    authorized: &Authorized,
    lamports: u64,
) -> Vec<Instruction> {
    let mut instructions = create_account(from_pubkey, stake_pubkey, authorized, lamports);
    instructions.push(delegate_stake(
        stake_pubkey,
        &authorized.staker,
        vote_pubkey,
    ));
    instructions
}

pub fn authorize(
    stake_pubkey: &Pubkey,
    authorized_pubkey: &Pubkey,
    new_authorized_pubkey: &Pubkey,
    stake_authorize: StakeAuthorize,
) -> Instruction {
    let account_metas = vec![
        AccountMeta::new(*stake_pubkey, false),
        AccountMeta::new_readonly(*authorized_pubkey, true),
    ];

    Instruction::new_with_bincode(
        STAKE_PROGRAM_ID,
        &StakeInstruction::Authorize(*new_authorized_pubkey, stake_authorize),
        account_metas,
    )
}

pub fn delegate_stake(
    stake_pubkey: &Pubkey,
    authorized_pubkey: &Pubkey,
    vote_pubkey: &Pubkey,
) -> Instruction {
    let account_metas = vec![
        AccountMeta::new(*stake_pubkey, false),
        AccountMeta::new_readonly(*vote_pubkey, false),
        AccountMeta::new_readonly(*authorized_pubkey, true),
    ];
    Instruction::new_with_bincode(
        STAKE_PROGRAM_ID,
        &StakeInstruction::DelegateStake,
        account_metas,
    )
}

pub fn withdraw(
    stake_pubkey: &Pubkey,
    withdrawer_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    lamports: u64,
) -> Instruction {
    let account_metas = vec![
        AccountMeta::new(*stake_pubkey, false),
        AccountMeta::new(*to_pubkey, false),
        AccountMeta::new_readonly(*withdrawer_pubkey, true),
    ];

    Instruction::new_with_bincode(
        STAKE_PROGRAM_ID,
        &StakeInstruction::Withdraw(lamports),
        account_metas,
    )
}

pub fn deactivate_stake(stake_pubkey: &Pubkey, authorized_pubkey: &Pubkey) -> Instruction {
    let account_metas = vec![
        AccountMeta::new(*stake_pubkey, false),
        // AccountMeta::new_readonly(sysvar::clock::STAKE_PROGRAM_ID, false),
        AccountMeta::new_readonly(*authorized_pubkey, true),
    ];
    Instruction::new_with_bincode(
        STAKE_PROGRAM_ID,
        &StakeInstruction::Deactivate,
        account_metas,
    )
}

// pub fn deactivate_delinquent_stake(
//     stake_account: &Pubkey,
//     delinquent_vote_account: &Pubkey,
//     reference_vote_account: &Pubkey,
// ) -> Instruction {
//     let account_metas = vec![
//         AccountMeta::new(*stake_account, false),
//         AccountMeta::new_readonly(*delinquent_vote_account, false),
//         AccountMeta::new_readonly(*reference_vote_account, false),
//     ];
//     Instruction::new_with_bincode(
//         STAKE_PROGRAM_ID,
//         &StakeInstruction::DeactivateDelinquent,
//         account_metas,
//     )
// }

// fn _redelegate(
//     stake_pubkey: &Pubkey,
//     authorized_pubkey: &Pubkey,
//     vote_pubkey: &Pubkey,
//     uninitialized_stake_pubkey: &Pubkey,
// ) -> Instruction {
//     let account_metas = vec![
//         AccountMeta::new(*stake_pubkey, false),
//         AccountMeta::new(*uninitialized_stake_pubkey, false),
//         AccountMeta::new_readonly(*vote_pubkey, false),
//         AccountMeta::new_readonly(*authorized_pubkey, true),
//     ];
//     Instruction::new_with_bincode(
//         STAKE_PROGRAM_ID,
//         &StakeInstruction::Redelegate,
//         account_metas,
//     )
// }

// pub fn redelegate(
//     stake_pubkey: &Pubkey,
//     authorized_pubkey: &Pubkey,
//     vote_pubkey: &Pubkey,
//     uninitialized_stake_pubkey: &Pubkey,
// ) -> Vec<Instruction> {
//     vec![
//         system_instruction::allocate(uninitialized_stake_pubkey, StakeStateV2::size_of() as u64),
//         system_instruction::assign(uninitialized_stake_pubkey, &STAKE_PROGRAM_ID),
//         _redelegate(
//             stake_pubkey,
//             authorized_pubkey,
//             vote_pubkey,
//             uninitialized_stake_pubkey,
//         ),
//     ]
// }
