//! Account container for migrating from one account type to another.

use crate::bpf_writer::BpfWriter;
use crate::error::{Error, ErrorCode};
use crate::solana_program::account_info::AccountInfo;
use crate::solana_program::instruction::AccountMeta;
use crate::solana_program::pubkey::Pubkey;
use crate::solana_program::system_program;
use crate::{
    AccountDeserialize, AccountSerialize, Accounts, AccountsExit, Key, Owner, Result,
    ToAccountInfos, ToAccountMetas,
};
use std::collections::BTreeSet;
use std::fmt;
use std::ops::{Deref, DerefMut};

/// Internal representation of the migration state.
enum MigrationInner<From, To> {
    /// Account is in old format, will be migrated and serialized on exit
    Old(From),
    /// Account is already in new format, will be serialized on exit
    New(Box<To>),
}

/// Wrapper around [`AccountInfo`](crate::solana_program::account_info::AccountInfo)
/// that handles account schema migrations from one type to another.
///
/// # Table of Contents
/// - [Basic Functionality](#basic-functionality)
/// - [Example](#example)
///
/// # Basic Functionality
///
/// `Migration` facilitates migrating account data from an old schema (`From`) to a new
/// schema (`To`). During deserialization, the account must be in the `From` format -
/// accounts already in the `To` format will be rejected with an error.
///
/// You must explicitly call `.migrate(new_data)` to perform the migration. The migrated
/// data is stored in memory and will be serialized to the account when the instruction exits.
///
/// On exit, the account must be in the migrated state or an error will be returned.
///
/// This type is typically used with the `realloc` constraint to resize the account
/// during migration.
///
/// Checks:
///
/// - `Account.info.owner == From::owner()`
/// - `!(Account.info.owner == SystemProgram && Account.info.lamports() == 0)`
/// - Account must deserialize as `From` (not `To`)
///
/// # Example
/// ```ignore
/// use anchor_lang::prelude::*;
///
/// declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
///
/// #[program]
/// pub mod my_program {
///     use super::*;
///
///     pub fn migrate(ctx: Context<MigrateAccount>) -> Result<()> {
///         // Perform migration
///         ctx.accounts.my_account.migrate(AccountV2 {
///             data: 42,
///             new_field: 0,
///         })?;
///
///         Ok(())
///     }
/// }
///
/// #[account]
/// pub struct AccountV1 {
///     pub data: u64,
/// }
///
/// #[account]
/// pub struct AccountV2 {
///     pub data: u64,
///     pub new_field: u64,
/// }
///
/// #[derive(Accounts)]
/// pub struct MigrateAccount<'info> {
///     #[account(mut)]
///     pub payer: Signer<'info>,
///     #[account(
///         mut,
///         realloc = 8 + AccountV2::INIT_SPACE,
///         realloc::payer = payer,
///         realloc::zero = false
///     )]
///     pub my_account: Migration<'info, AccountV1, AccountV2>,
///     pub system_program: Program<'info, System>,
/// }
/// ```
pub struct Migration<'info, From, To>
where
    From: AccountDeserialize,
    To: AccountSerialize,
{
    /// Account info reference
    info: &'info AccountInfo<'info>,
    /// Internal migration state
    inner: MigrationInner<From, To>,
}

impl<'info, From, To> fmt::Debug for Migration<'info, From, To>
where
    From: AccountDeserialize + Clone + fmt::Debug,
    To: AccountSerialize + Clone + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner_debug = match &self.inner {
            MigrationInner::Old(from) => format!("Old({:?})", from),
            MigrationInner::New(to) => format!("New({:?})", to),
        };
        f.debug_struct("Migration")
            .field("inner", &inner_debug)
            .field("info", &self.info)
            .finish()
    }
}

impl<'info, From, To> Migration<'info, From, To>
where
    From: AccountDeserialize + Owner,
    To: AccountSerialize + Owner,
{
    fn new(info: &'info AccountInfo<'info>, account: From) -> Self {
        Self {
            info,
            inner: MigrationInner::Old(account),
        }
    }

    /// Returns `true` if the account has been migrated.
    #[inline(always)]
    pub fn is_migrated(&self) -> bool {
        matches!(self.inner, MigrationInner::New(_))
    }

    /// Migrates the account by providing the new data.
    ///
    /// This method stores the new data in memory. The data will be
    /// serialized to the account when the instruction exits.
    ///
    /// # Errors
    /// Returns an error if the account has already been migrated.
    pub fn migrate(&mut self, new_data: To) -> Result<()> {
        if self.is_migrated() {
            return Err(ErrorCode::AccountAlreadyMigrated.into());
        }

        self.inner = MigrationInner::New(Box::new(new_data));
        Ok(())
    }

    /// Deserializes the given `info` into a `Migration`.
    ///
    /// Only accepts accounts in the `From` format. Accounts already in the `To`
    /// format will be rejected.
    #[inline(never)]
    pub fn try_from(info: &'info AccountInfo<'info>) -> Result<Self> {
        if info.owner == &system_program::ID && info.lamports() == 0 {
            return Err(ErrorCode::AccountNotInitialized.into());
        }

        if info.owner != &From::owner() {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*info.owner, From::owner())));
        }

        let mut data: &[u8] = &info.try_borrow_data()?;
        Ok(Self::new(info, From::try_deserialize(&mut data)?))
    }

    /// Deserializes the given `info` into a `Migration` without checking
    /// the account discriminator.
    ///
    /// **Warning:** Use with caution. This skips discriminator validation.
    #[inline(never)]
    pub fn try_from_unchecked(info: &'info AccountInfo<'info>) -> Result<Self> {
        if info.owner == &system_program::ID && info.lamports() == 0 {
            return Err(ErrorCode::AccountNotInitialized.into());
        }

        if info.owner != &From::owner() {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*info.owner, From::owner())));
        }

        let mut data: &[u8] = &info.try_borrow_data()?;
        Ok(Self::new(
            info,
            From::try_deserialize_unchecked(&mut data)?,
        ))
    }
}

impl<'info, B, From, To> Accounts<'info, B> for Migration<'info, From, To>
where
    From: AccountDeserialize + Owner,
    To: AccountSerialize + Owner,
{
    #[inline(never)]
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &'info [AccountInfo<'info>],
        _ix_data: &[u8],
        _bumps: &mut B,
        _reallocs: &mut BTreeSet<Pubkey>,
    ) -> Result<Self> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        Self::try_from(account)
    }
}

impl<'info, From, To> AccountsExit<'info> for Migration<'info, From, To>
where
    From: AccountDeserialize + Owner,
    To: AccountSerialize + Owner,
{
    fn exit(&self, program_id: &Pubkey) -> Result<()> {
        // Check if account is closed
        if crate::common::is_closed(self.info) {
            return Ok(());
        }

        // Check that the account has been migrated and serialize
        match &self.inner {
            MigrationInner::Old(_) => {
                // Account was not migrated - this is an error
                return Err(ErrorCode::AccountNotMigrated.into());
            }
            MigrationInner::New(to) => {
                // Only persist if the owner is the current program
                let expected_owner = To::owner();
                if &expected_owner != program_id {
                    return Ok(());
                }

                // Serialize the migrated data
                let mut data = self.info.try_borrow_mut_data()?;
                let dst: &mut [u8] = &mut data;
                let mut writer = BpfWriter::new(dst);
                to.as_ref().try_serialize(&mut writer)?;
            }
        }

        Ok(())
    }
}

impl<From, To> ToAccountMetas for Migration<'_, From, To>
where
    From: AccountDeserialize,
    To: AccountSerialize,
{
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.info.is_signer);
        let meta = match self.info.is_writable {
            false => AccountMeta::new_readonly(*self.info.key, is_signer),
            true => AccountMeta::new(*self.info.key, is_signer),
        };
        vec![meta]
    }
}

impl<'info, From, To> ToAccountInfos<'info> for Migration<'info, From, To>
where
    From: AccountDeserialize,
    To: AccountSerialize,
{
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.info.clone()]
    }
}

impl<'info, From, To> AsRef<AccountInfo<'info>> for Migration<'info, From, To>
where
    From: AccountDeserialize,
    To: AccountSerialize,
{
    fn as_ref(&self) -> &AccountInfo<'info> {
        self.info
    }
}

impl<From, To> Key for Migration<'_, From, To>
where
    From: AccountDeserialize,
    To: AccountSerialize,
{
    fn key(&self) -> Pubkey {
        *self.info.key
    }
}
