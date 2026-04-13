//! Prelude: import everything you need with `use anchor_lang_v2::prelude::*;`

pub use crate::{
    account,
    // Account types
    accounts::{
        Account, AccountValidate, BorshAccount, Optional, Program, Signer, SystemAccount, Sysvar,
        SysvarId, UncheckedAccount,
    },
    // CPI
    create_account,
    create_account_signed,
    create_program_address,
    // ID
    declare_id,
    emit,
    // Event
    event,
    find_program_address,
    // Msg
    msg,
    parse_instruction,
    // Pod types
    pod::{PodBool, PodI128, PodI16, PodI32, PodI64, PodU128, PodU16, PodU32, PodU64, PodVec},
    program,
    // Programs
    programs::{System, Token, Token2022},
    // Require macros (re-exported via #[macro_export])
    require,
    require_eq,
    require_gt,
    require_gte,
    require_keys_eq,
    require_keys_neq,
    require_neq,
    run_handler,
    // Hash
    sha256,
    sol_log_data,
    // Loader & dispatch
    AccountLoader,
    // Client
    AccountMeta as AnchorAccountMeta,
    // Derive macros
    Accounts,
    // Core trait
    AnchorAccount,
    AnchorDeserialize,
    // Serialization
    AnchorSerialize,
    Bumps,
    // Constraints
    Constrain,
    // Context
    Context,
    Discriminator,
    Error,
    // Re-export ProgramError for custom error impls
    Error as ProgramError,
    ErrorCode,
    Event,
    Id,
    InstructionData,
    // Nested
    Nested,
    // Marker traits
    Owner,
    // Error
    Result,
    ToAccountMetas,
    TryAccounts,
};
// Re-export pinocchio sysvar types and trait for use with Sysvar<T>
pub use pinocchio::sysvars::Sysvar as PinocchioSysvar;
pub use pinocchio::{
    account::AccountView,
    address::Address,
    sysvars::{clock::Clock, rent::Rent},
    ProgramResult,
};
