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
/// - [Smart Migration](#smart-migration)
/// - [Migration Modes](#migration-modes)
/// - [Example](#example)
///
/// # Basic Functionality
///
/// `Migration` facilitates migrating account data from an old schema (`From`) to a new
/// schema (`To`). The account can be in either the old or new format when deserialized.
/// You must explicitly call `.migrate(new_data)` to perform the migration. The `migrate()`
/// method is idempotent - calling it on an already-migrated account is a no-op.
///
/// On exit, the account must be in the migrated state or an error will be returned.
///
/// This type is typically used with the `realloc` constraint to resize the account
/// during migration.
///
/// Checks:
///
/// - `Account.info.owner == From::owner()` or `Account.info.owner == To::owner()` (smart mode)
/// - `!(Account.info.owner == SystemProgram && Account.info.lamports() == 0)`
///
/// # Smart Migration
///
/// By default, `Migration` uses smart detection when both `From` and `To` implement
/// the required traits. It attempts to deserialize as `To` first (already migrated),
/// falling back to `From` (needs migration). This allows gradual migrations where
/// some accounts may already be in the new format.
///
/// # Migration Modes
///
/// Use `migrate = "strict"` to reject accounts that are already migrated:
/// ```ignore
/// #[account(mut, migrate = "strict")]
/// pub my_account: Migration<'info, AccountV1, AccountV2>,
/// ```
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
///         // Access old data via deref
///         let old_data = ctx.accounts.my_account.data;
///
///         // Perform migration
///         ctx.accounts.my_account.migrate(AccountV2 {
///             data: old_data,
///             new_field: 0,
///         })?;
///
///         // Access new data after migration
///         let new = ctx.accounts.my_account.as_new()?;
///         msg!("Migrated! New field: {}", new.new_field);
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
    /// Internal migration state - either Old(From) or New(Box<To>)
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
    From: AccountDeserialize,
    To: AccountSerialize,
{
    fn new(info: &'info AccountInfo<'info>, account: From) -> Self {
        Self {
            info,
            inner: MigrationInner::Old(account),
        }
    }

    fn new_already_migrated(info: &'info AccountInfo<'info>, account: To) -> Self {
        Self {
            info,
            inner: MigrationInner::New(Box::new(account)),
        }
    }

    /// Migrates the account by providing the new data.
    ///
    /// This method is idempotent - if the account is already migrated, it returns `Ok(())`
    /// without modifying the account. Use the `migrate = "strict"` constraint if you want
    /// to ensure the account has not been migrated yet.
    ///
    /// # Example
    /// ```ignore
    /// pub fn migrate_account(ctx: Context<MigrateAccount>) -> Result<()> {
    ///     // Access old data via deref
    ///     let old_data = ctx.accounts.my_account.data;
    ///
    ///     // This will no-op if already migrated
    ///     ctx.accounts.my_account.migrate(AccountV2 {
    ///         data: old_data,
    ///         new_field: 0,
    ///     })?;
    ///
    ///     // Can access new data after migration
    ///     let new = ctx.accounts.my_account.as_new()?;
    ///     msg!("Migrated! New field: {}", new.new_field);
    ///     Ok(())
    /// }
    /// ```
    pub fn migrate(&mut self, new_data: To) -> Result<()> {
        match &self.inner {
            MigrationInner::Old(_) => {
                self.inner = MigrationInner::New(Box::new(new_data));
                Ok(())
            }
            MigrationInner::New(_) => {
                // Already migrated - no-op
                Ok(())
            }
        }
    }

    /// Access old data before migration.
    ///
    /// This method is used internally by the `Deref` implementation.
    /// Users should access old data via dereferencing instead.
    ///
    /// # Errors
    /// Returns an error if the account is already migrated.
    #[doc(hidden)]
    pub fn as_old(&self) -> Result<&From> {
        match &self.inner {
            MigrationInner::Old(from) => Ok(from),
            MigrationInner::New(_) => Err(ErrorCode::AccountAlreadyMigrated.into()),
        }
    }

    /// Access old data mutably before migration.
    ///
    /// This method is used internally by the `DerefMut` implementation.
    /// Users should access old data via dereferencing instead.
    ///
    /// # Errors
    /// Returns an error if the account is already migrated.
    #[doc(hidden)]
    pub fn as_old_mut(&mut self) -> Result<&mut From> {
        match &mut self.inner {
            MigrationInner::Old(from) => Ok(from),
            MigrationInner::New(_) => Err(ErrorCode::AccountAlreadyMigrated.into()),
        }
    }

    /// Access new data after migration.
    ///
    /// # Errors
    /// Returns an error if the account has not been migrated yet.
    pub fn as_new(&self) -> Result<&To> {
        match &self.inner {
            MigrationInner::New(to) => Ok(to.as_ref()),
            MigrationInner::Old(_) => Err(ErrorCode::AccountNotMigrated.into()),
        }
    }

    /// Access new data mutably after migration.
    ///
    /// # Errors
    /// Returns an error if the account has not been migrated yet.
    pub fn as_new_mut(&mut self) -> Result<&mut To> {
        match &mut self.inner {
            MigrationInner::New(to) => Ok(to.as_mut()),
            MigrationInner::Old(_) => Err(ErrorCode::AccountNotMigrated.into()),
        }
    }

    /// Returns `true` if the account was already in the new format when deserialized.
    pub fn is_already_migrated(&self) -> bool {
        matches!(self.inner, MigrationInner::New(_))
    }
}

impl<'info, From, To> Migration<'info, From, To>
where
    From: AccountDeserialize + Owner,
    To: AccountSerialize + AccountDeserialize + Owner,
{
    /// Deserializes the given `info` into a `Migration`.
    ///
    /// Uses smart detection: tries to deserialize as `To` (already migrated) first,
    /// falls back to `From` (needs migration).
    #[inline(never)]
    pub fn try_from(info: &'info AccountInfo<'info>) -> Result<Self> {
        Self::try_from_smart(info)
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

        // Try to deserialize as To (already migrated) without discriminator check
        let data_borrowed = info.try_borrow_data()?;
        let mut data: &[u8] = &data_borrowed;
        if info.owner == &To::owner() {
            if let Ok(already_migrated) = To::try_deserialize_unchecked(&mut data) {
                drop(data_borrowed);
                return Ok(Self::new_already_migrated(info, already_migrated));
            }
        }
        drop(data_borrowed);

        // Fall back to deserializing as From
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

impl<'info, From, To> Migration<'info, From, To>
where
    From: AccountDeserialize + Owner,
    To: AccountSerialize + AccountDeserialize + Owner,
{
    /// Reloads the account from storage. This is useful, for example, when
    /// observing side effects after CPI.
    ///
    /// Uses smart migration logic to detect if account is already in `To` format.
    pub fn reload(&mut self) -> Result<()> {
        if self.info.owner == &system_program::ID && self.info.lamports() == 0 {
            return Err(ErrorCode::AccountNotInitialized.into());
        }

        let data_borrowed = self.info.try_borrow_data()?;

        // Try to deserialize as To (already migrated) if owner matches
        if self.info.owner == &To::owner() {
            let mut cursor: &[u8] = &data_borrowed;
            if let Ok(already_migrated) = To::try_deserialize(&mut cursor) {
                drop(data_borrowed);
                self.inner = MigrationInner::New(Box::new(already_migrated));
                return Ok(());
            }
        }

        // Fall back to deserializing as From (data still borrowed)
        if self.info.owner != &From::owner() {
            drop(data_borrowed);
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*self.info.owner, From::owner())));
        }

        let mut cursor: &[u8] = &data_borrowed;
        let from = From::try_deserialize(&mut cursor)?;
        drop(data_borrowed);

        self.inner = MigrationInner::Old(from);
        Ok(())
    }

    /// Smart migration: tries to deserialize as `To` (already migrated) first,
    /// falls back to `From` (needs migration).
    ///
    /// This method is automatically called by the `Accounts` trait implementation when both
    /// `From` and `To` implement all required traits.
    /// You typically don't need to call this directly unless you're manually constructing the Migration.
    ///
    /// Use this for gradual migrations where some accounts may already be migrated.
    ///
    /// # Example
    /// ```ignore
    /// // This works with both AccountV1 and AccountV2 accounts
    /// let migration = Migration::<AccountV1, AccountV2>::try_from_smart(account_info)?;
    ///
    /// if migration.is_already_migrated() {
    ///     msg!("Account already migrated");
    /// }
    /// ```
    ///
    /// # Note
    /// When using the `#[account(migrate = "strict")]` constraint, accounts that are already
    /// in the `To` format will be rejected at the constraint validation stage.
    #[inline(never)]
    pub fn try_from_smart(info: &'info AccountInfo<'info>) -> Result<Self> {
        if info.owner == &system_program::ID && info.lamports() == 0 {
            return Err(ErrorCode::AccountNotInitialized.into());
        }

        // Try to deserialize as To (already migrated)
        let data_borrowed = info.try_borrow_data()?;
        let mut data: &[u8] = &data_borrowed;
        if info.owner == &To::owner() {
            if let Ok(already_migrated) = To::try_deserialize(&mut data) {
                // Account is already migrated!
                drop(data_borrowed);
                return Ok(Self::new_already_migrated(info, already_migrated));
            }
        }
        drop(data_borrowed);

        // Fall back to deserializing as From (needs migration)
        let mut data: &[u8] = &info.try_borrow_data()?;
        if info.owner != &From::owner() {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*info.owner, From::owner())));
        }
        Ok(Self::new(info, From::try_deserialize(&mut data)?))
    }
}

// Smart migration by default when To implements all required traits
impl<'info, B, From, To> Accounts<'info, B> for Migration<'info, From, To>
where
    From: AccountDeserialize + Owner,
    To: AccountSerialize + AccountDeserialize + Owner,
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
        Self::try_from_smart(account)
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

// Deref to From when account is in Old state
impl<'info, From, To> Deref for Migration<'info, From, To>
where
    From: AccountDeserialize,
    To: AccountSerialize,
{
    type Target = From;

    fn deref(&self) -> &Self::Target {
        self.as_old()
            .expect("Cannot deref to From: account is already migrated. Use .as_new() to access the migrated data.")
    }
}

// DerefMut to From when account is in Old state
impl<'info, From, To> DerefMut for Migration<'info, From, To>
where
    From: AccountDeserialize,
    To: AccountSerialize,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_old_mut()
            .expect("Cannot deref_mut to From: account is already migrated. Use .as_new_mut() to access the migrated data.")
    }
}
