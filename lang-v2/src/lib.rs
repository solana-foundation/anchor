//! Anchor v2: Trait-based account system for Solana.
//!
//! `#![no_std]` compatible. Depends only on pinocchio, borsh, bytemuck.

#![no_std]

extern crate alloc;

pub mod accounts;
mod context;
mod cpi;
pub mod cursor;
mod dispatch;
pub mod event;
pub mod hash;
pub mod loader;
pub mod pod;
pub mod prelude;
pub mod programs;
mod traits;

// Re-export derive macros
// Re-export borsh and bytemuck for generated code
#[cfg(feature = "account-resize")]
pub use cpi::realloc_account;
/// Chunked 4×u64 equality compare for `Address`. Preferred over `==`
/// on `&Address`. See <https://github.com/anza-xyz/solana-sdk/issues/345>.
pub use pinocchio::address::address_eq;
/// Re-export declare_id from solana-address.
pub use solana_address::declare_id;
/// Re-export log macro for generated code.
pub use solana_program_log::log as msg;
// Re-export wincode for instruction data serialization
pub use wincode;
pub use {
    accounts::{AccountInitialize, SlabInit},
    anchor_derive_accounts_v2::{
        access_control, account, constant, emit, event, program, Accounts,
    },
    borsh::{self, BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize},
    bytemuck,
    context::{Bumps, Context},
    cpi::{
        create_account, create_account_signed, create_program_address,
        find_and_verify_program_address, find_and_verify_program_address_skip_curve,
        find_program_address, verify_program_address,
    },
    cursor::{AccountBitvec, AccountCursor},
    dispatch::{parse_instruction, run_handler, TryAccounts},
    event::{sol_log_data, Event},
    hash::sha256,
    loader::AccountLoader,
    pinocchio::{account::AccountView, address::Address},
    traits::*,
};

// ---------------------------------------------------------------------------
// Client-side types — for building instructions off-chain (tests, CPI, SDK)
// ---------------------------------------------------------------------------

/// Metadata for a single account in a transaction instruction.
pub struct AccountMeta {
    pub address: Address,
    pub is_writable: bool,
    pub is_signer: bool,
}

/// Converts a struct of account addresses into a list of [`AccountMeta`]s.
pub trait ToAccountMetas {
    fn to_account_metas(&self, is_signer: Option<bool>) -> alloc::vec::Vec<AccountMeta>;
}

/// Serializes instruction data: discriminator prefix + LE-encoded args.
pub trait InstructionData: Discriminator {
    fn data(&self) -> alloc::vec::Vec<u8>;
}

/// Result type.
pub type Result<T> = core::result::Result<T, solana_program_error::ProgramError>;

/// Error type — just ProgramError for no_std.
pub type Error = solana_program_error::ProgramError;

/// Error codes matching Anchor v1's ErrorCode variants.
/// Used by constraint codegen.
pub enum ErrorCode {
    AccountNotEnoughKeys,
    ConstraintMut,
    ConstraintSigner,
    ConstraintSeeds,
    ConstraintHasOne,
    ConstraintAddress,
    ConstraintClose,
    ConstraintOwner,
    ConstraintRaw,
    ConstraintExecutable,
    ConstraintRentExempt,
    ConstraintZero,
    InstructionDidNotDeserialize,
    DeclaredProgramIdMismatch,
    InstructionFallbackNotFound,
    RequireViolated,
    RequireEqViolated,
    RequireNeqViolated,
    RequireKeysEqViolated,
    RequireKeysNeqViolated,
    RequireGtViolated,
    RequireGteViolated,
    ConstraintDuplicateMutableAccount,
}

impl From<ErrorCode> for solana_program_error::ProgramError {
    #[cold]
    #[inline(never)]
    fn from(e: ErrorCode) -> Self {
        match e {
            ErrorCode::AccountNotEnoughKeys => {
                solana_program_error::ProgramError::NotEnoughAccountKeys
            }
            ErrorCode::ConstraintMut => solana_program_error::ProgramError::Custom(2000),
            ErrorCode::ConstraintSigner => {
                solana_program_error::ProgramError::MissingRequiredSignature
            }
            ErrorCode::ConstraintSeeds => solana_program_error::ProgramError::InvalidSeeds,
            ErrorCode::ConstraintHasOne => solana_program_error::ProgramError::InvalidAccountData,
            ErrorCode::ConstraintAddress => solana_program_error::ProgramError::InvalidAccountData,
            ErrorCode::ConstraintClose => solana_program_error::ProgramError::InvalidAccountData,
            ErrorCode::ConstraintOwner => solana_program_error::ProgramError::IllegalOwner,
            ErrorCode::ConstraintRaw => solana_program_error::ProgramError::Custom(2001),
            ErrorCode::ConstraintExecutable => solana_program_error::ProgramError::Custom(2002),
            ErrorCode::ConstraintRentExempt => solana_program_error::ProgramError::Custom(2003),
            ErrorCode::ConstraintZero => solana_program_error::ProgramError::Custom(2004),
            ErrorCode::InstructionDidNotDeserialize => {
                solana_program_error::ProgramError::InvalidInstructionData
            }
            ErrorCode::DeclaredProgramIdMismatch => {
                solana_program_error::ProgramError::IncorrectProgramId
            }
            ErrorCode::InstructionFallbackNotFound => {
                solana_program_error::ProgramError::InvalidInstructionData
            }
            ErrorCode::RequireViolated => solana_program_error::ProgramError::Custom(2500),
            ErrorCode::RequireEqViolated => solana_program_error::ProgramError::Custom(2501),
            ErrorCode::RequireNeqViolated => solana_program_error::ProgramError::Custom(2502),
            ErrorCode::RequireKeysEqViolated => solana_program_error::ProgramError::Custom(2503),
            ErrorCode::RequireKeysNeqViolated => solana_program_error::ProgramError::Custom(2504),
            ErrorCode::RequireGtViolated => solana_program_error::ProgramError::Custom(2505),
            ErrorCode::RequireGteViolated => solana_program_error::ProgramError::Custom(2506),
            ErrorCode::ConstraintDuplicateMutableAccount => {
                solana_program_error::ProgramError::Custom(2005)
            }
        }
    }
}

/// Check if an account is rent-exempt. Used by `rent_exempt = enforce` constraint.
pub fn is_rent_exempt(view: &pinocchio::account::AccountView) -> bool {
    use pinocchio::sysvars::rent::{ACCOUNT_STORAGE_OVERHEAD, DEFAULT_LAMPORTS_PER_BYTE};
    let required = (ACCOUNT_STORAGE_OVERHEAD + view.data_len() as u64) * DEFAULT_LAMPORTS_PER_BYTE;
    view.lamports() >= required
}

/// Guardrail: verify that the runtime-supplied `program_id` matches this
/// program's `declare_id!()`. Gated behind the `guardrails` feature —
/// when disabled, compiles away entirely.
#[inline(always)]
pub fn check_program_id(
    _program_id: &Address,
    _declared: &Address,
) -> core::result::Result<(), solana_program_error::ProgramError> {
    #[cfg(feature = "guardrails")]
    if _program_id != _declared {
        return Err(ErrorCode::DeclaredProgramIdMismatch.into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// require! macros — no_std compatible
// ---------------------------------------------------------------------------

/// Ensures a condition is true, otherwise returns an error.
///
/// Can be used with or without a custom error code.
///
/// # Example
/// ```rust,ignore
/// require!(amount > 0, ErrorCode::ConstraintRaw);
/// require!(amount > 0, MyError::InvalidAmount);
/// ```
#[macro_export]
macro_rules! require {
    ($invariant:expr, $error:tt $(,)?) => {
        if !($invariant) {
            return Err($crate::ErrorCode::$error.into());
        }
    };
    ($invariant:expr, $error:expr $(,)?) => {
        if !($invariant) {
            return Err(core::convert::Into::into($error));
        }
    };
}

/// Ensures two NON-PUBKEY values are equal.
///
/// Use [require_keys_eq] to compare two pubkeys/addresses.
///
/// # Example
/// ```rust,ignore
/// require_eq!(ctx.accounts.data.count, 0);
/// require_eq!(ctx.accounts.data.count, 0, MyError::InvalidCount);
/// ```
#[macro_export]
macro_rules! require_eq {
    ($value1:expr, $value2:expr, $error_code:expr $(,)?) => {
        if $value1 != $value2 {
            $crate::msg!(
                "require_eq violation: left = {}, right = {}",
                $value1,
                $value2
            );
            return Err(core::convert::Into::into($error_code));
        }
    };
    ($value1:expr, $value2:expr $(,)?) => {
        if $value1 != $value2 {
            $crate::msg!(
                "require_eq violation: left = {}, right = {}",
                $value1,
                $value2
            );
            return Err($crate::ErrorCode::RequireEqViolated.into());
        }
    };
}

/// Ensures two NON-PUBKEY values are not equal.
///
/// Use [require_keys_neq] to compare two pubkeys/addresses.
///
/// # Example
/// ```rust,ignore
/// require_neq!(ctx.accounts.data.count, 0);
/// require_neq!(ctx.accounts.data.count, 0, MyError::InvalidCount);
/// ```
#[macro_export]
macro_rules! require_neq {
    ($value1:expr, $value2:expr, $error_code:expr $(,)?) => {
        if $value1 == $value2 {
            $crate::msg!(
                "require_neq violation: left = {}, right = {}",
                $value1,
                $value2
            );
            return Err(core::convert::Into::into($error_code));
        }
    };
    ($value1:expr, $value2:expr $(,)?) => {
        if $value1 == $value2 {
            $crate::msg!(
                "require_neq violation: left = {}, right = {}",
                $value1,
                $value2
            );
            return Err($crate::ErrorCode::RequireNeqViolated.into());
        }
    };
}

/// Ensures two pubkey/address values are equal.
///
/// Use [require_eq] to compare two non-pubkey values.
///
/// # Example
/// ```rust,ignore
/// require_keys_eq!(*ctx.accounts.data.authority(), ctx.accounts.authority.key());
/// ```
#[macro_export]
macro_rules! require_keys_eq {
    ($value1:expr, $value2:expr, $error_code:expr $(,)?) => {
        if $value1 != $value2 {
            $crate::msg!("require_keys_eq violation");
            return Err(core::convert::Into::into($error_code));
        }
    };
    ($value1:expr, $value2:expr $(,)?) => {
        if $value1 != $value2 {
            $crate::msg!("require_keys_eq violation");
            return Err($crate::ErrorCode::RequireKeysEqViolated.into());
        }
    };
}

/// Ensures two pubkey/address values are not equal.
///
/// Use [require_neq] to compare two non-pubkey values.
///
/// # Example
/// ```rust,ignore
/// require_keys_neq!(*ctx.accounts.data.authority(), ctx.accounts.other.key());
/// ```
#[macro_export]
macro_rules! require_keys_neq {
    ($value1:expr, $value2:expr, $error_code:expr $(,)?) => {
        if $value1 == $value2 {
            $crate::msg!("require_keys_neq violation");
            return Err(core::convert::Into::into($error_code));
        }
    };
    ($value1:expr, $value2:expr $(,)?) => {
        if $value1 == $value2 {
            $crate::msg!("require_keys_neq violation");
            return Err($crate::ErrorCode::RequireKeysNeqViolated.into());
        }
    };
}

/// Ensures the first value is greater than the second.
///
/// # Example
/// ```rust,ignore
/// require_gt!(ctx.accounts.data.count, 0);
/// require_gt!(ctx.accounts.data.count, 0, MyError::InvalidCount);
/// ```
#[macro_export]
macro_rules! require_gt {
    ($value1:expr, $value2:expr, $error_code:expr $(,)?) => {
        if $value1 <= $value2 {
            $crate::msg!(
                "require_gt violation: left = {}, right = {}",
                $value1,
                $value2
            );
            return Err(core::convert::Into::into($error_code));
        }
    };
    ($value1:expr, $value2:expr $(,)?) => {
        if $value1 <= $value2 {
            $crate::msg!(
                "require_gt violation: left = {}, right = {}",
                $value1,
                $value2
            );
            return Err($crate::ErrorCode::RequireGtViolated.into());
        }
    };
}

/// Ensures the first value is greater than or equal to the second.
///
/// # Example
/// ```rust,ignore
/// require_gte!(ctx.accounts.data.count, 1);
/// require_gte!(ctx.accounts.data.count, 1, MyError::InvalidCount);
/// ```
#[macro_export]
macro_rules! require_gte {
    ($value1:expr, $value2:expr, $error_code:expr $(,)?) => {
        if $value1 < $value2 {
            $crate::msg!(
                "require_gte violation: left = {}, right = {}",
                $value1,
                $value2
            );
            return Err(core::convert::Into::into($error_code));
        }
    };
    ($value1:expr, $value2:expr $(,)?) => {
        if $value1 < $value2 {
            $crate::msg!(
                "require_gte violation: left = {}, right = {}",
                $value1,
                $value2
            );
            return Err($crate::ErrorCode::RequireGteViolated.into());
        }
    };
}
