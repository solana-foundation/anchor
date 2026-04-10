//! Anchor v2: Trait-based account system for Solana.
//!
//! `#![no_std]` compatible. Depends only on pinocchio, borsh, bytemuck.

#![no_std]

extern crate alloc;

pub mod accounts;
mod context;
mod cpi;
pub mod event;
pub mod prelude;
pub mod programs;
mod traits;

pub use pinocchio::account::AccountView;
pub use pinocchio::address::Address;
pub use context::Context;
pub use cpi::{create_account, create_account_signed, find_program_address};
pub use traits::*;
pub use event::{Event, sol_log_data};

/// Re-export msg for generated code.
pub use solana_msg::msg;

/// Re-export declare_id from solana-address.
pub use solana_address::declare_id;

// Re-export derive macros
pub use anchor_derive_accounts_v2::Accounts;
pub use anchor_derive_accounts_v2::account;
pub use anchor_derive_accounts_v2::program;
pub use anchor_derive_accounts_v2::event;
pub use anchor_derive_accounts_v2::emit;

// Re-export borsh and bytemuck for generated code
pub use borsh::{self, BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use bytemuck;

/// Result type.
pub type Result<T> = core::result::Result<T, solana_program_error::ProgramError>;

/// Error type — just ProgramError for no_std.
pub type Error = solana_program_error::ProgramError;

/// Error codes matching Anchor v1's ErrorCode variants.
/// Used by constraint codegen.
pub enum ErrorCode {
    AccountNotEnoughKeys,
    ConstraintSeeds,
    ConstraintHasOne,
    ConstraintAddress,
    ConstraintRaw,
    InstructionDidNotDeserialize,
    DeclaredProgramIdMismatch,
    InstructionFallbackNotFound,
}

impl From<ErrorCode> for solana_program_error::ProgramError {
    fn from(e: ErrorCode) -> Self {
        match e {
            ErrorCode::AccountNotEnoughKeys => solana_program_error::ProgramError::NotEnoughAccountKeys,
            ErrorCode::ConstraintSeeds => solana_program_error::ProgramError::InvalidSeeds,
            ErrorCode::ConstraintHasOne => solana_program_error::ProgramError::InvalidAccountData,
            ErrorCode::ConstraintAddress => solana_program_error::ProgramError::InvalidAccountData,
            ErrorCode::ConstraintRaw => solana_program_error::ProgramError::Custom(2000),
            ErrorCode::InstructionDidNotDeserialize => solana_program_error::ProgramError::InvalidInstructionData,
            ErrorCode::DeclaredProgramIdMismatch => solana_program_error::ProgramError::IncorrectProgramId,
            ErrorCode::InstructionFallbackNotFound => solana_program_error::ProgramError::InvalidInstructionData,
        }
    }
}
