use num_derive::{FromPrimitive, ToPrimitive};
use thiserror::Error;

use crate::account::AccountMeta;
use crate::decode_error::DecodeError;
use crate::instruction::Instruction;
use crate::pubkey::Pubkey;
use crate::system_program::SYSTEM_PROGRAM_ID;

#[derive(Error, Debug, Serialize, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum SystemError {
    #[error("an account with the same address already exists")]
    AccountAlreadyInUse,
    #[error("account does not have enough SOL to perform the operation")]
    ResultWithNegativeLamports,
    #[error("cannot assign account to this program id")]
    InvalidProgramId,
    #[error("cannot allocate account data of this length")]
    InvalidAccountDataLength,
    #[error("length of requested seed is too long")]
    MaxSeedLengthExceeded,
    #[error("provided address does not match addressed derived from seed")]
    AddressWithSeedMismatch,
    #[error("advancing stored nonce requires a populated RecentBlockhashes sysvar")]
    NonceNoRecentBlockhashes,
    #[error("stored nonce is still in recent_blockhashes")]
    NonceBlockhashNotExpired,
    #[error("specified nonce does not match stored nonce")]
    NonceUnexpectedBlockhashValue,
}

impl<T> DecodeError<T> for SystemError {
    fn type_of() -> &'static str {
        "SystemError"
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SystemInstruction {
    /// Create a new account
    ///
    /// # Account references
    ///   0. `[WRITE, SIGNER]` Funding account
    ///   1. `[WRITE, SIGNER]` New account
    CreateAccount {
        /// Number of lamports to transfer to the new account
        lamports: u64,

        /// Number of bytes of memory to allocate
        space: u64,

        /// Address of program that will own the new account
        owner: Pubkey,
    },

    /// Create a new account with an anchor
    ///
    /// # Account references
    ///   0. `[WRITE, SIGNER]` Funding account
    ///   1. `[WRITE, SIGNER]` New account
    CreateAccountWithAnchor {
        /// Number of lamports to transfer to the new account
        lamports: u64,

        /// Number of bytes of memory to allocate
        space: u64,

        /// Address of program that will own the new account
        owner: Pubkey,

        /// UTXO to be anchored to the new account
        txid: [u8; 32],
        vout: u32,
    },

    /// Anchor an account to a utxo
    ///
    /// # Account references
    ///   0. `[WRITE, SIGNER]` Assigned account public key
    Assign {
        /// Owner program account
        owner: Pubkey,
    },

    /// Assign account to a program
    ///
    /// # Account references
    ///   0. `[WRITE, SIGNER]` Assigned account public key
    Anchor {
        /// UTXO to be anchored to the new account
        txid: [u8; 32],
        vout: u32,
    },

    /// Transfer lamports
    ///
    /// # Account references
    ///   0. `[WRITE, SIGNER]` Funding account
    ///   1. `[WRITE]` Recipient account
    Transfer { lamports: u64 },

    /// Allocate space in a (possibly new) account without funding
    ///
    /// # Account references
    ///   0. `[WRITE, SIGNER]` New account
    Allocate {
        /// Number of bytes of memory to allocate
        space: u64,
    },

    /// Consumes a stored nonce, replacing it with a successor
    ///
    /// # Account references
    ///   0. `[WRITE]` Nonce account
    ///   1. `[]` RecentBlockhashes sysvar
    ///   2. `[SIGNER]` Nonce authority
    AdvanceNonceAccount,

    /// Withdraw funds from a nonce account
    ///
    /// # Account references
    ///   0. `[WRITE]` Nonce account
    ///   1. `[WRITE]` Recipient account
    ///   2. `[]` RecentBlockhashes sysvar
    ///   3. `[]` Rent sysvar
    ///   4. `[SIGNER]` Nonce authority
    ///
    /// The `u64` parameter is the lamports to withdraw, which must leave the
    /// account balance above the rent exempt reserve or at zero.
    WithdrawNonceAccount(u64),

    /// Drive state of Uninitialized nonce account to Initialized, setting the nonce value
    ///
    /// # Account references
    ///   0. `[WRITE]` Nonce account
    ///   1. `[]` RecentBlockhashes sysvar
    ///   2. `[]` Rent sysvar
    ///
    /// The `Pubkey` parameter specifies the entity authorized to execute nonce
    /// instruction on the account
    ///
    /// No signatures are required to execute this instruction, enabling derived
    /// nonce account addresses
    InitializeNonceAccount(Pubkey),

    /// Change the entity authorized to execute nonce instructions on the account
    ///
    /// # Account references
    ///   0. `[WRITE]` Nonce account
    ///   1. `[SIGNER]` Nonce authority
    ///
    /// The `Pubkey` parameter identifies the entity to authorize
    AuthorizeNonceAccount(Pubkey),
}

/// Creates a new account instruction linked to a specific UTXO.
///
/// This instruction will create a new account in the system identified by the given
/// transaction ID and output index (txid, vout).
///
/// # Parameters
/// * `txid` - The transaction ID as a 32-byte array
/// * `vout` - The output index
/// * `pubkey` - The public key that will own the new account
///
/// # Returns
/// * `Instruction` - The system instruction to create the account
pub fn create_account(
    from_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
) -> Instruction {
    let account_metas = vec![
        AccountMeta::new(*from_pubkey, true),
        AccountMeta::new(*to_pubkey, true),
    ];
    Instruction::new_with_bincode(
        SYSTEM_PROGRAM_ID,
        &SystemInstruction::CreateAccount {
            lamports,
            space,
            owner: *owner,
        },
        account_metas,
    )
}

pub fn create_account_with_anchor(
    from_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
    txid: [u8; 32],
    vout: u32,
) -> Instruction {
    let account_metas = vec![
        AccountMeta::new(*from_pubkey, true),
        AccountMeta::new(*to_pubkey, true),
    ];
    Instruction::new_with_bincode(
        SYSTEM_PROGRAM_ID,
        &SystemInstruction::CreateAccountWithAnchor {
            lamports,
            space,
            owner: *owner,
            txid,
            vout,
        },
        account_metas,
    )
}

pub fn create_account_with_seed(
    from_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    base: &Pubkey,
    _seed: &str,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
) -> Instruction {
    // For Arch, seeds are not required. We include `base` as a readonly account so that
    // existing callers can continue to pass it but otherwise fall back to the standard
    // `CreateAccount` variant.
    let account_metas = vec![
        AccountMeta::new(*from_pubkey, true),
        AccountMeta::new(*to_pubkey, true),
        AccountMeta::new_readonly(*base, false),
    ];
    Instruction::new_with_bincode(
        SYSTEM_PROGRAM_ID,
        &SystemInstruction::CreateAccount {
            lamports,
            space,
            owner: *owner,
        },
        account_metas,
    )
}

/// Assigns a new owner to an account.
///
/// This instruction changes the owner of an account, which determines
/// which program has authority to modify the account.
///
/// # Parameters
/// * `pubkey` - The public key of the account to be reassigned
/// * `owner` - The public key of the new owner (program)
///
/// # Returns
/// * `Instruction` - The system instruction to assign a new owner
pub fn assign(pubkey: &Pubkey, owner: &Pubkey) -> Instruction {
    let account_metas = vec![AccountMeta::new(*pubkey, true)];
    Instruction::new_with_bincode(
        SYSTEM_PROGRAM_ID,
        &SystemInstruction::Assign { owner: *owner },
        account_metas,
    )
}

pub fn transfer(from_pubkey: &Pubkey, to_pubkey: &Pubkey, lamports: u64) -> Instruction {
    let account_metas = vec![
        AccountMeta::new(*from_pubkey, true),
        AccountMeta::new(*to_pubkey, false),
    ];
    Instruction::new_with_bincode(
        SYSTEM_PROGRAM_ID,
        &SystemInstruction::Transfer { lamports },
        account_metas,
    )
}

pub fn allocate(pubkey: &Pubkey, space: u64) -> Instruction {
    let account_metas = vec![AccountMeta::new(*pubkey, true)];
    Instruction::new_with_bincode(
        SYSTEM_PROGRAM_ID,
        &SystemInstruction::Allocate { space },
        account_metas,
    )
}

pub fn anchor(pubkey: &Pubkey, txid: [u8; 32], vout: u32) -> Instruction {
    let account_metas = vec![AccountMeta::new(*pubkey, true)];
    Instruction::new_with_bincode(
        SYSTEM_PROGRAM_ID,
        &SystemInstruction::Anchor { txid, vout },
        account_metas,
    )
}

// pub fn create_nonce_account(
//     from_pubkey: &Pubkey,
//     nonce_pubkey: &Pubkey,
//     authority: &Pubkey,
//     lamports: u64,
//     txid: [u8; 32],
//     vout: u32,
// ) -> Vec<Instruction> {
//     vec![
//         create_account(
//             from_pubkey,
//             nonce_pubkey,
//             lamports,
//             nonce::State::size() as u64,
//             &SYSTEM_PROGRAM_ID,
//             txid,
//             vout,
//         ),
//         Instruction::new_with_bincode(
//             SYSTEM_PROGRAM_ID,
//             &SystemInstruction::InitializeNonceAccount(*authority),
//             vec![
//                 AccountMeta::new(*nonce_pubkey, false),
//                 #[allow(deprecated)]
//                 AccountMeta::new_readonly(recent_blockhashes::id(), false),
//             ],
//         ),
//     ]
// }

// pub fn advance_nonce_account(nonce_pubkey: &Pubkey, authorized_pubkey: &Pubkey) -> Instruction {
//     let account_metas = vec![
//         AccountMeta::new(*nonce_pubkey, false),
//         #[allow(deprecated)]
//         AccountMeta::new_readonly(recent_blockhashes::id(), false),
//         AccountMeta::new_readonly(*authorized_pubkey, true),
//     ];
//     Instruction::new_with_bincode(
//         SYSTEM_PROGRAM_ID,
//         &SystemInstruction::AdvanceNonceAccount,
//         account_metas,
//     )
// }

// pub fn withdraw_nonce_account(
//     nonce_pubkey: &Pubkey,
//     authorized_pubkey: &Pubkey,
//     to_pubkey: &Pubkey,
//     lamports: u64,
// ) -> Instruction {
//     let account_metas = vec![
//         AccountMeta::new(*nonce_pubkey, false),
//         AccountMeta::new(*to_pubkey, false),
//         #[allow(deprecated)]
//         AccountMeta::new_readonly(recent_blockhashes::id(), false),
//         AccountMeta::new_readonly(*authorized_pubkey, true),
//     ];
//     Instruction::new_with_bincode(
//         SYSTEM_PROGRAM_ID,
//         &SystemInstruction::WithdrawNonceAccount(lamports),
//         account_metas,
//     )
// }

// pub fn authorize_nonce_account(
//     nonce_pubkey: &Pubkey,
//     authorized_pubkey: &Pubkey,
//     new_authority: &Pubkey,
// ) -> Instruction {
//     let account_metas = vec![
//         AccountMeta::new(*nonce_pubkey, false),
//         AccountMeta::new_readonly(*authorized_pubkey, true),
//     ];
//     Instruction::new_with_bincode(
//         SYSTEM_PROGRAM_ID,
//         &SystemInstruction::AuthorizeNonceAccount(*new_authority),
//         account_metas,
//     )
// }
