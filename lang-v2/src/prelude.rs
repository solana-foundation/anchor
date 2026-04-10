//! Prelude: import everything you need with `use anchor_lang_v2::prelude::*;`

pub use crate::{
    // Core trait
    AnchorAccount,
    // Marker traits
    Owner, Id, Discriminator,
    // Account types
    accounts::{
        Account, AccountValidate, BorshAccount, Optional, Program, Signer,
        SystemAccount, UncheckedAccount, Sysvar, SysvarId,
        token::{TokenAccount, Mint},
    },
    // Programs
    programs::{System, Token, Token2022},
    // Context
    Context, Bumps,
    // Loader & dispatch
    AccountLoader, TryAccounts, run_handler, parse_instruction,
    // CPI
    create_account, create_account_signed, find_program_address, create_program_address,
    // Derive macros
    Accounts, account, program,
    // Event
    event, emit, Event, sol_log_data,
    // Serialization
    AnchorSerialize, AnchorDeserialize,
    // Error
    Result, Error, ErrorCode,
    // Re-export ProgramError for custom error impls
    Error as ProgramError,
    // Constants
    DISC_LEN,
    // Constraints
    constraints::Constrain,
    // Nested
    Nested,
    // Client
    AccountMeta as AnchorAccountMeta, InstructionData, ToAccountMetas,
    // Msg
    msg,
    // ID
    declare_id,
    // Pod types
    pod::{
        PodU16, PodU32, PodU64, PodU128,
        PodI16, PodI32, PodI64, PodI128,
        PodBool, PodVec,
    },
    // Require macros (re-exported via #[macro_export])
    require, require_eq, require_neq,
    require_keys_eq, require_keys_neq,
    require_gt, require_gte,
};

pub use pinocchio::account::AccountView;
pub use pinocchio::address::Address;
pub use pinocchio::ProgramResult;

// Re-export pinocchio sysvar types and trait for use with Sysvar<T>
pub use pinocchio::sysvars::Sysvar as PinocchioSysvar;
pub use pinocchio::sysvars::clock::Clock;
pub use pinocchio::sysvars::rent::Rent;
