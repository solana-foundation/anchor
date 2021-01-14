use solana_sdk::account_info::AccountInfo;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::program_error::ProgramError;
use solana_sdk::pubkey::Pubkey;
use std::io::Write;
use std::ops::{Deref, DerefMut};

pub use anchor_attribute_access_control::access_control;
pub use anchor_attribute_account::account;
pub use anchor_attribute_program::program;
pub use anchor_derive_accounts::Accounts;
/// Default serialization format for anchor instructions and accounts.
pub use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};

/// A data structure of Solana accounts that can be deserialized from the input
/// of a Solana program. Due to the freewheeling nature of the accounts array,
/// implementations of this trait should perform any and all constraint checks
/// (in addition to any done within `AccountDeserialize`) on accounts to ensure
/// the accounts maintain any invariants required for the program to run
/// securely.
pub trait Accounts<'info>: ToAccountMetas + ToAccountInfos<'info> + Sized {
    fn try_accounts(
        program_id: &Pubkey,
        from: &mut &[AccountInfo<'info>],
    ) -> Result<Self, ProgramError>;
}

/// Transformation to `AccountMeta` structs.
pub trait ToAccountMetas {
    fn to_account_metas(&self) -> Vec<AccountMeta>;
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

/// Container for a serializable `account`. Use this to reference any account
/// owned by the currently executing program.
#[derive(Clone)]
pub struct ProgramAccount<'a, T: AccountSerialize + AccountDeserialize + Clone> {
    info: AccountInfo<'a>,
    account: T,
}

impl<'a, T: AccountSerialize + AccountDeserialize + Clone> ProgramAccount<'a, T> {
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

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> ToAccountInfo<'info>
    for ProgramAccount<'info, T>
{
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.info.clone()
    }
}

impl<'a, T: AccountSerialize + AccountDeserialize + Clone> Deref for ProgramAccount<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.account
    }
}

impl<'a, T: AccountSerialize + AccountDeserialize + Clone> DerefMut for ProgramAccount<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.account
    }
}

/// Similar to `ProgramAccount`, but to reference any account *not* owned by
/// the current program.
pub type CpiAccount<'a, T> = ProgramAccount<'a, T>;

/// Container for a Solana sysvar.
pub struct Sysvar<'info, T: solana_sdk::sysvar::Sysvar> {
    info: AccountInfo<'info>,
    account: T,
}

impl<'info, T: solana_sdk::sysvar::Sysvar> Sysvar<'info, T> {
    pub fn from_account_info(
        acc_info: &AccountInfo<'info>,
    ) -> Result<Sysvar<'info, T>, ProgramError> {
        Ok(Sysvar {
            info: acc_info.clone(),
            account: T::from_account_info(&acc_info)?,
        })
    }
}

impl<'a, T: solana_sdk::sysvar::Sysvar> Deref for Sysvar<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.account
    }
}

impl<'a, T: solana_sdk::sysvar::Sysvar> DerefMut for Sysvar<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.account
    }
}

impl<'info, T: solana_sdk::sysvar::Sysvar> ToAccountInfo<'info> for Sysvar<'info, T> {
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.info.clone()
    }
}

impl<'info> ToAccountInfo<'info> for AccountInfo<'info> {
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.clone()
    }
}

/// Provides non-argument inputs to the program.
pub struct Context<'a, 'b, 'c, 'info, T> {
    /// Deserialized accounts.
    pub accounts: &'a mut T,
    /// Currently executing program id.
    pub program_id: &'b Pubkey,
    /// Remaining accounts given but not deserialized or validated.
    pub remaining_accounts: &'c [AccountInfo<'info>],
}

impl<'a, 'b, 'c, 'info, T> Context<'a, 'b, 'c, 'info, T> {
    pub fn new(
        accounts: &'a mut T,
        program_id: &'b Pubkey,
        remaining_accounts: &'c [AccountInfo<'info>],
    ) -> Self {
        Self {
            accounts,
            program_id,
            remaining_accounts,
        }
    }
}

/// Context speciying non-argument inputs for cross-program-invocations.
pub struct CpiContext<'a, 'b, 'c, 'info, T: Accounts<'info>> {
    pub accounts: T,
    pub program: AccountInfo<'info>,
    pub signer_seeds: &'a [&'b [&'c [u8]]],
}

impl<'a, 'b, 'c, 'info, T: Accounts<'info>> CpiContext<'a, 'b, 'c, 'info, T> {
    pub fn new(program: AccountInfo<'info>, accounts: T) -> Self {
        Self {
            accounts,
            program,
            signer_seeds: &[],
        }
    }

    pub fn new_with_signer(
        accounts: T,
        program: AccountInfo<'info>,
        signer_seeds: &'a [&'b [&'c [u8]]],
    ) -> Self {
        Self {
            accounts,
            program,
            signer_seeds,
        }
    }
}

pub mod prelude {
    pub use super::{
        access_control, account, program, AccountDeserialize, AccountSerialize, Accounts,
        AnchorDeserialize, AnchorSerialize, Context, CpiAccount, CpiContext, ProgramAccount,
        Sysvar, ToAccountInfo, ToAccountInfos, ToAccountMetas,
    };

    pub use solana_program::msg;
    pub use solana_sdk::account_info::{next_account_info, AccountInfo};
    pub use solana_sdk::entrypoint::ProgramResult;
    pub use solana_sdk::instruction::AccountMeta;
    pub use solana_sdk::program_error::ProgramError;
    pub use solana_sdk::pubkey::Pubkey;
    pub use solana_sdk::sysvar::clock::Clock;
    pub use solana_sdk::sysvar::epoch_schedule::EpochSchedule;
    pub use solana_sdk::sysvar::fees::Fees;
    pub use solana_sdk::sysvar::instructions::Instructions;
    pub use solana_sdk::sysvar::recent_blockhashes::RecentBlockhashes;
    pub use solana_sdk::sysvar::rent::Rent;
    pub use solana_sdk::sysvar::rewards::Rewards;
    pub use solana_sdk::sysvar::slot_hashes::SlotHashes;
    pub use solana_sdk::sysvar::slot_history::SlotHistory;
    pub use solana_sdk::sysvar::stake_history::StakeHistory;
    pub use solana_sdk::sysvar::Sysvar as SolanaSysvar;
}
