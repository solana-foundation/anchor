//! Prelude: import everything you need with `use anchor_lang_v2::prelude::*;`

pub use crate::{
    // Core trait
    AnchorAccount, AnchorAccountInit,
    // Marker traits
    Owner, Id, Discriminator,
    // Account types
    accounts::{
        Account, BorshAccount, Optional, Program, Signer,
        SystemAccount, UncheckedAccount,
    },
    // Programs
    programs::System,
    // Context
    Context,
    // CPI
    create_account, create_account_signed, find_program_address,
    // Derive macros
    Accounts, AnchorData, account, program,
    // Serialization
    AnchorSerialize, AnchorDeserialize,
    // Error
    Result, Error, ErrorCode,
    // Constants
    DISC_LEN,
    // Nested
    Nested,
    // Msg
    msg,
};

pub use pinocchio::account::AccountView;
pub use pinocchio::address::Address;
pub use pinocchio::ProgramResult;
