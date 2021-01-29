//! Anchor ⚓ is a framework for Solana's Sealevel runtime providing several
//! convenient developer tools.
//!
//! - Rust eDSL for writing safe, secure, and high level Solana programs
//! - [IDL](https://en.wikipedia.org/wiki/Interface_description_language) specification
//! - TypeScript package for generating clients from IDL
//! - CLI and workspace management for developing complete applications
//!
//! If you're familiar with developing in Ethereum's
//! [Solidity](https://docs.soliditylang.org/en/v0.7.4/),
//! [Truffle](https://www.trufflesuite.com/),
//! [web3.js](https://github.com/ethereum/web3.js) or Parity's
//! [Ink!](https://github.com/paritytech/ink), then the experience will be
//! familiar. Although the syntax and semantics are targeted at Solana, the high
//! level workflow of writing RPC request handlers, emitting an IDL, and
//! generating clients from IDL is the same.
//!
//! For detailed tutorials and examples on how to use Anchor, see the guided
//! [tutorials](https://project-serum.github.io/anchor) or examples in the GitHub
//! [repository](https://github.com/project-serum/anchor).
//!
//! Presented here are the Rust primitives for building on Solana.

use solana_program::account_info::AccountInfo;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::io::Write;

mod account_info;
mod boxed;
mod context;
mod cpi_account;
mod ctor;
mod error;
pub mod idl;
mod program_account;
mod state;
mod sysvar;

pub use crate::context::{Context, CpiContext};
pub use crate::cpi_account::CpiAccount;
pub use crate::ctor::Ctor;
pub use crate::program_account::ProgramAccount;
pub use crate::state::ProgramState;
pub use crate::sysvar::Sysvar;
pub use anchor_attribute_access_control::access_control;
pub use anchor_attribute_account::account;
pub use anchor_attribute_error::error;
pub use anchor_attribute_program::program;
pub use anchor_attribute_state::state;
pub use anchor_derive_accounts::Accounts;
/// Default serialization format for anchor instructions and accounts.
pub use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use error::Error;
pub use solana_program;

/// A data structure of accounts that can be deserialized from the input
/// of a Solana program. Due to the freewheeling nature of the accounts array,
/// implementations of this trait should perform any and all constraint checks
/// (in addition to any done within `AccountDeserialize`) on accounts to ensure
/// the accounts maintain any invariants required for the program to run
/// securely.
pub trait Accounts<'info>: ToAccountMetas + ToAccountInfos<'info> + Sized {
    fn try_accounts(
        program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
    ) -> Result<Self, ProgramError>;
}

/// The exit procedure for an accounts object.
pub trait AccountsExit<'info>: ToAccountMetas + ToAccountInfos<'info> {
    fn exit(&self, program_id: &Pubkey) -> solana_program::entrypoint::ProgramResult;
}

/// A data structure of accounts providing a one time deserialization upon
/// initialization, i.e., when the data array for a given account is zeroed.
/// For all subsequent deserializations, it's expected that
/// [Accounts](trait.Accounts.html) is used.
pub trait AccountsInit<'info>: ToAccountMetas + ToAccountInfos<'info> + Sized {
    fn try_accounts_init(
        program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
    ) -> Result<Self, ProgramError>;
}

/// Transformation to `AccountMeta` structs.
pub trait ToAccountMetas {
    /// `is_signer` is given as an optional override for the signer meta field.
    /// This covers the edge case when a program-derived-address needs to relay
    /// a transaction from a client to another program but sign the transaction
    /// before the relay. The client cannot mark the field as a signer, and so
    /// we have to override the is_signer meta field given by the client.
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta>;
}

/// Transformation to `AccountInfo` structs.
pub trait ToAccountInfos<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>>;
}

/// Transformation to an `AccountInfo` struct.
pub trait ToAccountInfo<'info> {
    fn to_account_info(&self) -> AccountInfo<'info>;
}

/// A data structure that can be serialized and stored in an `AccountInfo` data
/// array.
///
/// Implementors of this trait should ensure that any subsequent usage the
/// `AccountDeserialize` trait succeeds if and only if the account is of the
/// correct type. For example, the implementation provided by the `#[account]`
/// attribute sets the first 8 bytes to be a unique account discriminator,
/// defined as the first 8 bytes of the SHA256 of the account's Rust ident.
/// Thus, any subsequent  calls via `AccountDeserialize`'s `try_deserialize`
/// will check this discriminator. If it doesn't match, an invalid account
/// was given, and the program will exit with an error.
pub trait AccountSerialize {
    /// Serilalizes the account data into `writer`.
    fn try_serialize<W: Write>(&self, writer: &mut W) -> Result<(), ProgramError>;
}

/// A data structure that can be deserialized from an `AccountInfo` data array.
pub trait AccountDeserialize: Sized {
    /// Deserializes the account data.
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError>;

    /// Deserializes account data without checking the account discriminator.
    /// This should only be used on account initialization, when the
    /// discriminator is not yet set (since the entire account data is zeroed).
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError>;
}

/// The prelude contains all commonly used components of the crate.
/// All programs should include it via `anchor_lang::prelude::*;`.
pub mod prelude {
    pub use super::{
        access_control, account, error, program, state, AccountDeserialize, AccountSerialize,
        Accounts, AccountsExit, AccountsInit, AnchorDeserialize, AnchorSerialize, Context,
        CpiAccount, CpiContext, Ctor, ProgramAccount, ProgramState, Sysvar, ToAccountInfo,
        ToAccountInfos, ToAccountMetas,
    };

    pub use borsh;
    pub use solana_program::account_info::{next_account_info, AccountInfo};
    pub use solana_program::entrypoint::ProgramResult;
    pub use solana_program::instruction::AccountMeta;
    pub use solana_program::msg;
    pub use solana_program::program_error::ProgramError;
    pub use solana_program::pubkey::Pubkey;
    pub use solana_program::sysvar::clock::Clock;
    pub use solana_program::sysvar::epoch_schedule::EpochSchedule;
    pub use solana_program::sysvar::fees::Fees;
    pub use solana_program::sysvar::instructions::Instructions;
    pub use solana_program::sysvar::recent_blockhashes::RecentBlockhashes;
    pub use solana_program::sysvar::rent::Rent;
    pub use solana_program::sysvar::rewards::Rewards;
    pub use solana_program::sysvar::slot_hashes::SlotHashes;
    pub use solana_program::sysvar::slot_history::SlotHistory;
    pub use solana_program::sysvar::stake_history::StakeHistory;
    pub use solana_program::sysvar::Sysvar as SolanaSysvar;
    pub use thiserror;
}
