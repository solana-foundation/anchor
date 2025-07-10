/// This module defines the instruction data structure and related error types.
/// Instructions are the fundamental unit of program execution in the Arch Network,
/// containing the program to call, accounts to interact with, and instruction data.
use std::mem::size_of;

#[cfg(feature = "fuzzing")]
use libfuzzer_sys::arbitrary;
use thiserror::Error;

use crate::program_error::*;
use crate::pubkey::Pubkey;
use crate::stake::instruction::StakeError;
use crate::system_instruction::SystemError;
use crate::vote::instruction::VoteError;
use crate::{account::AccountMeta, program_error::ProgramError};

use bitcode::{Decode, Encode};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use sha256::digest;

/// An instruction for a program to execute.
///
/// Each instruction contains:
/// * The program ID (public key) of the program that should process this instruction
/// * A list of accounts that the instruction will operate on
/// * A byte array of instruction data that is program-specific
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Encode,
    Decode,
)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub struct Instruction {
    /// Program ID that executes this instruction
    pub program_id: Pubkey,
    /// Metadata for accounts that this instruction interacts with
    pub accounts: Vec<AccountMeta>,
    /// Program-specific instruction data
    pub data: Vec<u8>,
}

impl Instruction {
    pub fn new(program_id: Pubkey, data: Vec<u8>, accounts: Vec<AccountMeta>) -> Self {
        Self {
            program_id,
            accounts,
            data,
        }
    }

    /// Serializes an instruction to a byte array.
    ///
    /// The serialization format is:
    /// * program_id (32 bytes)
    /// * accounts length (1 byte)
    /// * account metadata (34 bytes per account)
    /// * data length (8 bytes)
    /// * instruction data (variable)
    ///
    /// # Returns
    /// A vector containing the serialized instruction
    pub fn serialize(&self) -> Vec<u8> {
        let mut serilized = vec![];

        serilized.extend(self.program_id.serialize());
        // accounts length should fit in a u8
        serilized.push(self.accounts.len() as u8);
        for meta in self.accounts.iter() {
            serilized.extend(&meta.serialize());
        }
        // data length should fit in a u64
        serilized.extend(self.data.len().to_le_bytes());
        serilized.extend(&self.data);

        serilized
    }

    /// Deserializes an instruction from a byte slice.
    ///
    /// # Parameters
    /// * `data` - The byte slice containing the serialized instruction
    ///
    /// # Returns
    /// A new Instruction instance
    pub fn from_slice(data: &[u8]) -> Self {
        let mut size = 32;
        let accounts_len = data[size] as usize;
        size += 1;
        let mut accounts = Vec::with_capacity(accounts_len);
        for _ in 0..accounts_len {
            accounts.push(AccountMeta::from_slice(&data[size..(size + 34)]));
            size += size_of::<AccountMeta>();
        }
        let data_len = u64::from_le_bytes(data[size..(size + 8)].try_into().unwrap());
        size += size_of::<u64>();

        Self {
            program_id: Pubkey::from_slice(&data[..32]),
            accounts,
            data: (data[size..(size + data_len as usize)]).to_vec(),
        }
    }

    /// Computes a unique hash for this instruction.
    ///
    /// # Returns
    /// A string containing the double SHA-256 hash of the serialized instruction
    pub fn hash(&self) -> String {
        digest(digest(self.serialize()))
    }

    pub fn new_with_bincode<T: Serialize>(
        program_id: Pubkey,
        data: T,
        accounts: Vec<AccountMeta>,
    ) -> Self {
        Self {
            program_id,
            accounts,
            data: bincode::serialize(&data).unwrap(),
        }
    }

    pub fn new_with_borsh<T: borsh::BorshSerialize>(
        program_id: Pubkey,
        data: &T,
        accounts: Vec<AccountMeta>,
    ) -> Self {
        let data = borsh::to_vec(data).unwrap();
        Self {
            program_id,
            accounts,
            data,
        }
    }
}

/// Errors that can be returned by programs when processing instructions.
///
/// These errors are used by the runtime to indicate various failure conditions
/// that may occur during instruction execution.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum InstructionError {
    /// Deprecated! Use CustomError instead!
    /// The program instruction returned an error
    #[error("generic instruction error")]
    GenericError,

    /// The arguments provided to a program were invalid
    #[error("invalid program argument")]
    InvalidArgument,

    /// An instruction's data contents were invalid
    #[error("invalid instruction data")]
    InvalidInstructionData,

    /// An account's data contents was invalid
    #[error("invalid account data for instruction")]
    InvalidAccountData,

    /// An account's data was too small
    #[error("account data too small for instruction")]
    AccountDataTooSmall,

    /// An account's balance was too small to complete the instruction
    #[error("insufficient funds for instruction")]
    InsufficientFunds,

    /// The account did not have the expected program id
    #[error("incorrect program id for instructions")]
    IncorrectProgramId,

    /// A signature was required but not found
    #[error("missing required signature for instruction")]
    MissingRequiredSignature,

    /// An initialize instruction was sent to an account that has already been initialized.
    #[error("instruction requires an uninitialized account")]
    AccountAlreadyInitialized,

    /// An attempt to operate on an account that hasn't been initialized.
    #[error("instruction requires an initialized account")]
    UninitializedAccount,

    /// Program's instruction lamport balance does not equal the balance after the instruction
    #[error("sum of account balances before and after instruction do not match")]
    UnbalancedInstruction,

    /// Program illegally modified an account's program id
    #[error("instruction illegally modified the program id of an account")]
    ModifiedProgramId,

    /// Program spent the lamports of an account that doesn't belong to it
    #[error("instruction spent from the balance of an account it does not own")]
    ExternalAccountLamportSpend,

    /// Program modified the data of an account that doesn't belong to it
    #[error("instruction modified data of an account it does not own, Account key: {0}, instruction program should own this account in order to modify it")]
    ExternalAccountDataModified(String),

    /// Read-only account's data was modified
    #[error("instruction modified data of a read-only account, Account key: {0}")]
    ReadonlyDataModified(String),

    /// An account was referenced more than once in a single instruction
    // Deprecated, instructions can now contain duplicate accounts
    #[error("instruction contains duplicate accounts")]
    DuplicateAccountIndex,

    /// Executable bit on account changed, but shouldn't have
    #[error("instruction changed executable bit of an account")]
    ExecutableModified,

    /// The instruction expected additional account keys
    #[error("insufficient account keys for instruction")]
    NotEnoughAccountKeys,

    /// Program other than the account's owner changed the size of the account data
    #[error("program other than the account's owner changed the size of the account data")]
    AccountDataSizeChanged,

    /// The instruction expected an executable account
    #[error("instruction expected an executable account")]
    AccountNotExecutable,

    /// Failed to borrow a reference to account data, already borrowed
    #[error("instruction tries to borrow reference for an account which is already borrowed")]
    AccountBorrowFailed,

    /// Account data has an outstanding reference after a program's execution
    #[error("instruction left account with an outstanding borrowed reference")]
    AccountBorrowOutstanding,

    /// The same account was multiply passed to an on-chain program's entrypoint, but the program
    /// modified them differently.  A program can only modify one instance of the account because
    /// the runtime cannot determine which changes to pick or how to merge them if both are modified
    #[error("instruction modifications of multiply-passed account differ")]
    DuplicateAccountOutOfSync,

    /// Allows on-chain programs to implement program-specific error types and see them returned
    /// by the Solana runtime. A program-specific error may be any type that is represented as
    /// or serialized to a u32 integer.
    #[error("custom program error: {0:#x}")]
    Custom(u32),

    /// Error caused during a processing of a program
    #[error("program error: {0}")]
    ProgramError(ProgramError),

    /// The return value from the program was invalid.  Valid errors are either a defined builtin
    /// error value or a user-defined error in the lower 32 bits.
    #[error("program returned invalid error code")]
    InvalidError,

    /// Executable account's data was modified
    #[error("instruction changed executable accounts data")]
    ExecutableDataModified,

    /// Unsupported program id
    #[error("Unsupported program id")]
    UnsupportedProgramId,

    /// Cross-program invocation call depth too deep
    #[error("Cross-program invocation call depth too deep")]
    CallDepth,

    /// An account required by the instruction is missing
    #[error("An account required by the instruction is missing")]
    MissingAccount,

    /// Cross-program invocation reentrancy not allowed for this instruction
    #[error("Cross-program invocation reentrancy not allowed for this instruction")]
    ReentrancyNotAllowed,

    /// Length of the seed is too long for address generation
    #[error("Length of the seed is too long for address generation")]
    MaxSeedLengthExceeded,

    /// Provided seeds do not result in a valid address
    #[error("Provided seeds do not result in a valid address")]
    InvalidSeeds,

    /// Failed to reallocate account data of this length
    #[error("Failed to reallocate account data")]
    InvalidRealloc,

    /// Computational budget exceeded
    #[error("Computational budget exceeded")]
    ComputationalBudgetExceeded,

    /// Cross-program invocation with unauthorized signer or writable account
    #[error("Cross-program invocation with unauthorized signer or writable account")]
    PrivilegeEscalation,

    /// Failed to create program execution environment
    #[error("Failed to create program execution environment")]
    ProgramEnvironmentSetupFailure,

    /// Program failed to complete
    #[error("Program failed to complete")]
    ProgramFailedToComplete,

    /// Program failed to compile
    #[error("Program failed to compile")]
    ProgramFailedToCompile,

    /// Program failed to compile
    #[error("Elf failed to parse")]
    ElfFailedToParse,

    /// Account is immutable
    #[error("Account is immutable")]
    Immutable,

    /// Incorrect authority provided
    #[error("Incorrect authority provided")]
    IncorrectAuthority,

    /// Failed to serialize or deserialize account data
    ///
    /// Warning: This error should never be emitted by the runtime.
    ///
    /// This error includes strings from the underlying 3rd party Borsh crate
    /// which can be dangerous because the error strings could change across
    /// Borsh versions. Only programs can use this error because they are
    /// consistent across Solana software versions.
    ///
    #[error("Failed to serialize or deserialize account data: {0}")]
    BorshIoError(String),

    /// Invalid account owner
    #[error("Invalid account owner")]
    InvalidAccountOwner,

    /// Program arithmetic overflowed
    #[error("Program arithmetic overflowed")]
    ArithmeticOverflow,

    /// Unsupported sysvar
    #[error("Unsupported sysvar")]
    UnsupportedSysvar,

    /// Illegal account owner
    #[error("Provided owner is not allowed")]
    IllegalOwner,

    /// Accounts data allocations exceeded the maximum allowed per transaction
    #[error("Accounts data allocations exceeded the maximum allowed per transaction")]
    MaxAccountsDataAllocationsExceeded,

    /// Max accounts exceeded
    #[error("Max accounts exceeded")]
    MaxAccountsExceeded,

    /// Max instruction trace length exceeded
    #[error("Max instruction trace length exceeded")]
    MaxInstructionTraceLengthExceeded,

    /// Error Initilising BTC RPC
    #[error("unable to connect to bitcoin rpc")]
    RPCError,

    /// Builtin programs must consume compute units
    #[error("Builtin programs must consume compute units")]
    BuiltinProgramsMustConsumeComputeUnits,

    /// Vm execution failed
    #[error("Vm failed while executing ebpf code {0}")]
    EbpfError(String),

    /// Invalid transaction to sign
    #[error("Invalid transaction to sign")]
    InvalidTxToSign,

    #[error("Error occured during running of System Program {0}")]
    SystemError(SystemError), // Note: For any new error added here an equivalent ProgramError and its
    // conversions must also be added
    #[error("Account lamports cannot be negative")]
    NegativeAccountLamports,

    #[error("Readonly lamport change")]
    ReadonlyLamportChange,

    #[error("Executable lamport change")]
    ExecutableLamportChange,

    #[error("Bitcoin encoding error")]
    BitcoinEncodingError,

    #[error("Titan error")]
    TitanError,

    #[error("Invalid utxo owner")]
    InvalidUtxoOwner,

    #[error("Stake error: {0}")]
    StakeError(StakeError),

    #[error("Vote error: {0}")]
    VoteError(VoteError),

    #[error("Account is not anchored")]
    AccountNotAnchored,
}

impl From<SystemError> for InstructionError {
    fn from(error: SystemError) -> Self {
        InstructionError::SystemError(error)
    }
}

/// Implementation for converting u64 error codes to InstructionError variants
#[allow(non_snake_case)]
impl From<u64> for InstructionError {
    /// Converts a u64 error code to the corresponding InstructionError variant.
    ///
    /// # Parameters
    /// * `value` - The u64 error code to convert
    ///
    /// # Returns
    /// The corresponding InstructionError variant
    fn from(value: u64) -> Self {
        match value {
            CUSTOM_ZERO => Self::Custom(0),
            INVALID_ARGUMENT => Self::InvalidArgument,
            INVALID_INSTRUCTION_DATA => Self::InvalidInstructionData,
            INVALID_ACCOUNT_DATA => Self::InvalidAccountData,
            ACCOUNT_DATA_TOO_SMALL => Self::AccountDataTooSmall,
            INSUFFICIENT_FUNDS => Self::InsufficientFunds,
            INCORRECT_PROGRAM_ID => Self::IncorrectProgramId,
            MISSING_REQUIRED_SIGNATURES => Self::MissingRequiredSignature,
            ACCOUNT_ALREADY_INITIALIZED => Self::AccountAlreadyInitialized,
            UNINITIALIZED_ACCOUNT => Self::UninitializedAccount,
            NOT_ENOUGH_ACCOUNT_KEYS => Self::NotEnoughAccountKeys,
            ACCOUNT_BORROW_FAILED => Self::AccountBorrowFailed,
            MAX_SEED_LENGTH_EXCEEDED => Self::MaxSeedLengthExceeded,
            INVALID_SEEDS => Self::InvalidSeeds,
            BORSH_IO_ERROR => Self::BorshIoError("Unknown".to_string()),
            UNSUPPORTED_SYSVAR => Self::UnsupportedSysvar,
            ILLEGAL_OWNER => Self::IllegalOwner,
            MAX_ACCOUNTS_DATA_ALLOCATIONS_EXCEEDED => Self::MaxAccountsDataAllocationsExceeded,
            INVALID_ACCOUNT_DATA_REALLOC => Self::InvalidRealloc,
            MAX_INSTRUCTION_TRACE_LENGTH_EXCEEDED => Self::MaxInstructionTraceLengthExceeded,
            BUILTIN_PROGRAMS_MUST_CONSUME_COMPUTE_UNITS => {
                Self::BuiltinProgramsMustConsumeComputeUnits
            }
            INVALID_ACCOUNT_OWNER => Self::InvalidAccountOwner,
            ARITHMETIC_OVERFLOW => Self::ArithmeticOverflow,
            IMMUTABLE => Self::Immutable,
            INCORRECT_AUTHORITY => Self::IncorrectAuthority,
            NEGATIVE_ACCOUNT_LAMPORTS => Self::NegativeAccountLamports,
            READONLY_LAMPORT_CHANGE => Self::ReadonlyLamportChange,
            EXECUTABLE_LAMPORT_CHANGE => Self::ExecutableLamportChange,
            ACCOUNT_NOT_ANCHORED => Self::AccountNotAnchored,
            _ => {
                // A valid custom error has no bits set in the upper 32
                if value >> BUILTIN_BIT_SHIFT == 0 {
                    Self::Custom(value as u32)
                } else {
                    Self::InvalidError
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{account::AccountMeta, pubkey::Pubkey};

    use super::Instruction;

    #[test]
    fn test_serialize_deserialize() {
        let instruction = Instruction {
            program_id: Pubkey::system_program(),
            accounts: vec![],
            data: vec![],
        };

        assert_eq!(
            instruction,
            Instruction::from_slice(&instruction.serialize())
        );

        let instruction = Instruction {
            program_id: Pubkey::system_program(),
            accounts: vec![AccountMeta {
                pubkey: Pubkey::system_program(),
                is_signer: true,
                is_writable: true,
            }],
            data: vec![10; 364],
        };

        assert_eq!(
            instruction,
            Instruction::from_slice(&instruction.serialize())
        );
    }

    #[test]
    fn test_error_converion_to_u64() {
        let error = UNINITIALIZED_ACCOUNT;
        let instruction_error = InstructionError::from(error);
        assert_eq!(instruction_error, InstructionError::UninitializedAccount);
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn fuzz_serialize_deserialize_instruction(
            program_id in prop::array::uniform32(any::<u8>()),
            account_pubkeys in prop::collection::vec(prop::array::uniform32(any::<u8>()), 0..10),
            is_signer_flags in prop::collection::vec(any::<bool>(), 0..10),
            is_writable_flags in prop::collection::vec(any::<bool>(), 0..10),
            data in prop::collection::vec(any::<u8>(), 0..1024)
        ) {
            let accounts: Vec<AccountMeta> = account_pubkeys.into_iter()
                .zip(is_signer_flags.into_iter())
                .zip(is_writable_flags.into_iter())
                .map(|((pubkey, is_signer), is_writable)| AccountMeta {
                    pubkey: Pubkey::from(pubkey),
                    is_signer,
                    is_writable,
                })
                .collect();

            let instruction = Instruction {
                program_id: Pubkey::from(program_id),
                accounts,
                data: data.clone(),
            };

            let serialized = instruction.serialize();
            let deserialized = Instruction::from_slice(&serialized);

            assert_eq!(instruction, deserialized);
        }
    }
}
