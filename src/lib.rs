use solana_sdk::account_info::AccountInfo;
use solana_sdk::program_error::ProgramError;
use solana_sdk::pubkey::Pubkey;
use std::io::Write;
use std::ops::{Deref, DerefMut};

pub use anchor_attribute_access_control::access_control;
pub use anchor_attribute_account::account;
pub use anchor_attribute_program::program;
pub use anchor_derive_accounts::Accounts;
pub use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};

/// A data structure of Solana accounts that can be deserialized from the input
/// of a Solana program. Due to the freewheeling nature of the accounts array,
/// implementations of this trait should perform any and all constraint checks
/// (in addition to any done within `AccountDeserialize`) on accounts to ensure
/// the accounts maintain any invariants required for the program to run
/// securely.
pub trait Accounts<'info>: Sized {
    fn try_accounts(program_id: &Pubkey, from: &[AccountInfo<'info>])
        -> Result<Self, ProgramError>;
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

/// A container for a deserialized `account` and raw `AccountInfo` object.
///
/// Using this within a data structure deriving `Accounts` will ensure the
/// account is owned by the currently executing program.
pub struct ProgramAccount<'a, T: AccountSerialize + AccountDeserialize> {
    pub info: AccountInfo<'a>,
    pub account: T,
}

impl<'a, T: AccountSerialize + AccountDeserialize> ProgramAccount<'a, T> {
    pub fn new(info: AccountInfo<'a>, account: T) -> ProgramAccount<'a, T> {
        Self { info, account }
    }

    /// Deserializes the given `info` into a `ProgramAccount`.
    pub fn try_from(info: &AccountInfo<'a>) -> Result<ProgramAccount<'a, T>, ProgramError> {
        let mut data: &[u8] = &info.try_borrow_data()?;
        Ok(ProgramAccount::new(
            info.clone(),
            T::try_deserialize(&mut data)?,
        ))
    }

    /// Deserializes the zero-initialized `info` into a `ProgramAccount` without
    /// checking the account type. This should only be used upon program account
    /// initialization (since the entire account data array is zeroed and thus
    /// no account type is set).
    pub fn try_from_init(info: &AccountInfo<'a>) -> Result<ProgramAccount<'a, T>, ProgramError> {
        let mut data: &[u8] = &info.try_borrow_data()?;

        // The discriminator should be zero, since we're initializing.
        let mut disc_bytes = [0u8; 8];
        disc_bytes.copy_from_slice(&data[..8]);
        let discriminator = u64::from_le_bytes(disc_bytes);
        if discriminator != 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(ProgramAccount::new(
            info.clone(),
            T::try_deserialize_unchecked(&mut data)?,
        ))
    }
}

impl<'a, T: AccountSerialize + AccountDeserialize> Deref for ProgramAccount<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.account
    }
}

impl<'a, T: AccountSerialize + AccountDeserialize> DerefMut for ProgramAccount<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.account
    }
}

/// A data structure providing non-argument inputs to the Solana program, namely
/// the currently executing program's ID and the set of validated, deserialized
/// accounts.
pub struct Context<'a, 'b, T> {
    pub accounts: &'a mut T,
    pub program_id: &'b Pubkey,
}

pub mod prelude {
    pub use super::{
        access_control, account, program, AccountDeserialize, AccountSerialize, Accounts,
        AnchorDeserialize, AnchorSerialize, Context, ProgramAccount,
    };

    pub use solana_program::msg;
    pub use solana_sdk::account_info::next_account_info;
    pub use solana_sdk::account_info::AccountInfo;
    pub use solana_sdk::entrypoint::ProgramResult;
    pub use solana_sdk::program_error::ProgramError;
    pub use solana_sdk::pubkey::Pubkey;
}
