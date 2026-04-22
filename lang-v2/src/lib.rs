//! Anchor v2: Trait-based account system for Solana.
//!
//! `#![no_std]` compatible.

#![no_std]

extern crate alloc;

pub mod accounts;
mod context;
pub mod cpi;
mod context_cpi;
pub mod cursor;
mod dispatch;
pub mod event;
pub mod hash;
#[cfg(feature = "idl-build")]
pub mod idl_build;
pub mod loader;
pub mod pod;
pub mod prelude;
pub mod programs;
#[cfg(feature = "testing")]
pub mod testing;
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
#[doc(hidden)]
pub use solana_program_log::log as __log_impl;

// Re-export for `debug!` macro — routes through this crate's namespace so
// user programs don't need `solana-program-log` or `extern crate alloc;`.
#[cfg(feature = "compat")]
#[doc(hidden)]
pub use solana_program_log::log as __log_str;

// Ungated re-export so generated macro code (`#[event]`, `debug!`, etc.)
// can reach `Vec` without std or `extern crate alloc;` in user crates.
#[doc(hidden)]
pub extern crate alloc as __alloc;

/// Logs a message via `solana_program_log`.
///
/// Thin wrapper around `solana_program_log::log!` that always evaluates to
/// `()`, so it's usable in expression position (match arms, closures, tuples,
/// etc.) where the underlying macro's trailing-semicolon emission would
/// otherwise produce a parse error.
#[macro_export]
macro_rules! msg {
    ($($arg:tt)*) => {{
        $crate::__log_impl!($($arg)*);
    }};
}

/// v1-compat logger with full `format!` support (`{:?}`, `{:x}`, etc.).
///
/// Heap-allocates via `alloc::format!`. Prefer [`msg!`] for production
/// (cheaper CUs). Gated behind `compat`.
///
/// # Example
///
/// ```ignore
/// debug!("raw bytes: {:?}", &data[..32]);
/// ```
#[cfg(feature = "compat")]
#[macro_export]
macro_rules! debug {
    ($msg:expr) => {{
        $crate::__log_str($msg)
    }};
    ($($arg:tt)*) => {{
        $crate::__log_str(&$crate::__alloc::format!($($arg)*))
    }};
}
// Re-export wincode for instruction data serialization
pub use wincode;

/// Borsh-compatible wincode config: u8 enum tags + fixed u32 LE length
/// prefixes. Used for all serialization in v2 (instruction args, events,
/// `BorshAccount<T>`) so the on-chain wire format matches borsh exactly,
/// while keeping wincode's faster encoding path.
///
/// `ZERO_COPY_ALIGN_CHECK = false`: borsh's u32 length prefix puts payload
/// data 4 bytes off natural alignment, so handler args like `amounts: &[u64]`
/// would otherwise fail wincode's runtime alignment guard. The guard exists
/// to prevent Rust-level UB on hosts where misaligned wide loads are
/// undefined; SBPF's `ldxdw` has no alignment-specialized variants and the
/// Solana program ecosystem already reads u64 from arbitrary `&[u8]` offsets,
/// so disabling the check on SBPF is safe.
///
/// Compatibility caveats:
/// - `HashMap` / `HashSet` are NOT byte-identical (borsh sorts by key,
///   wincode preserves insertion order). Use `Vec<(K, V)>` if you need
///   canonical ordering.
/// - `f32` / `f64` NaN: borsh rejects on deserialize, wincode accepts.
pub const BORSH_CONFIG: wincode::config::Configuration<
    false,
    { wincode::config::DEFAULT_PREALLOCATION_SIZE_LIMIT },
    wincode::len::FixIntLen<u32>,
    wincode::int_encoding::LittleEndian,
    wincode::int_encoding::FixInt,
    u8,
> = wincode::config::Configuration::new();

// Internal: only used by `#[cfg(feature = "idl-build")]` codegen from the
// derive macros to split type-def JSON in `__anchor_private_print_idl_program`.
// Not part of the stable API — hence the `__` prefix.
#[cfg(feature = "idl-build")]
#[doc(hidden)]
pub use serde_json as __serde_json;
pub use {
    accounts::{AccountInitialize, SlabInit},
    anchor_derive_accounts_v2::{
        access_control, account, constant, emit, error_code, event, pod_wrapper, program,
        Accounts, InitSpace,
    },
    borsh::{self, BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize},
    bytemuck,
    context::{Bumps, Context},
    cpi::{
        create_account, create_account_signed, create_program_address,
        find_and_verify_program_address, find_and_verify_program_address_skip_curve,
        find_program_address, verify_program_address,
    },
    context_cpi::CpiContext,
    cursor::{mut_mask_or_shifted, mut_mask_set_bit, AccountBitvec, AccountCursor},
    dispatch::{run_handler, TryAccounts},
    event::{sol_log_data, Event},
    hash::sha256,
    loader::AccountLoader,
    pinocchio::{self, account::AccountView, address::Address},
    traits::*,
};

#[cfg(feature = "idl-build")]
pub use idl_build::IdlAccountType;
/// `#[derive(IdlType)]` — register a plain struct in the IDL's `types[]`
/// array. Always exported; the emitted impl body is itself
/// `#[cfg(feature = "idl-build")]`, so non-IDL builds pay nothing.
pub use anchor_derive_accounts_v2::IdlType;

// ---------------------------------------------------------------------------
// Client-side types — for building instructions off-chain (tests, CPI, SDK)
// ---------------------------------------------------------------------------

/// Metadata for a single account in a transaction instruction.
///
/// Re-exported from `solana-instruction` so tests and CPI builders can pass
/// the output of `to_account_metas()` straight into `solana_instruction::
/// Instruction::new_with_bytes` without a manual field rename.
pub use solana_instruction::account_meta::AccountMeta;

/// Re-export of the Solana SDK `Instruction` + `AccountMeta` types under a v1-
/// compatible module path. Lets users write
/// `use anchor_lang_v2::solana_program::instruction::{Instruction, AccountMeta}`
/// without adding `solana-instruction` to their `Cargo.toml`.
pub mod solana_program {
    pub mod instruction {
        pub use solana_instruction::*;
    }
}

/// Converts a struct of account addresses into a list of [`AccountMeta`]s.
pub trait ToAccountMetas {
    fn to_account_metas(&self, is_signer: Option<bool>) -> alloc::vec::Vec<AccountMeta>;
}

/// Serializes instruction data: discriminator prefix + LE-encoded args.
pub trait InstructionData: Discriminator {
    fn data(&self) -> alloc::vec::Vec<u8>;
}

/// Compile-time account-size calculation. Derived via `#[derive(InitSpace)]`.
/// Typically used to size account rent: `space = 8 + MyAccount::INIT_SPACE`.
///
/// The derive handles Borsh-size accounting for variable-length fields via a
/// `#[max_len(N)]` helper attribute on `String` / `Vec<T>` fields. POD accounts
/// that use the default wincode backing should just use `core::mem::size_of`.
pub trait Space {
    const INIT_SPACE: usize;
}

#[doc(hidden)]
pub mod __private {
    /// Used by `#[derive(InitSpace)]` on enums to pick the largest variant size.
    pub const fn max(a: usize, b: usize) -> usize {
        [a, b][(a < b) as usize]
    }
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

/// Guardrail: verify the runtime-supplied account count doesn't exceed
/// `__ANCHOR_MAX_ACCOUNTS` (256). Gated behind the `guardrails` feature —
/// when disabled, compiles away entirely.
#[inline(always)]
pub fn check_max_accounts(
    _num: usize,
    _max: usize,
) -> core::result::Result<(), solana_program_error::ProgramError> {
    #[cfg(feature = "guardrails")]
    if _num > _max {
        return Err(ErrorCode::AccountNotEnoughKeys.into());
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
/// require_keys_eq!(*ctx.accounts.data.authority(), *ctx.accounts.authority.address());
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
/// require_keys_neq!(*ctx.accounts.data.authority(), *ctx.accounts.other.address());
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
