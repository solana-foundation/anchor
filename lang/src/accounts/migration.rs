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
pub enum MigrationInner<From, To> {
    /// Account is in old format, will be migrated and serialized on exit
    From(From),
    /// Account is already in new format, will be serialized on exit
    To(To),
}

/// Wrapper around [`AccountInfo`](crate::solana_program::account_info::AccountInfo)
/// that handles account schema migrations from one type to another.
///
/// # Table of Contents
/// - [Basic Functionality](#basic-functionality)
/// - [Usage Patterns](#usage-patterns)
/// - [Example](#example)
///
/// # Basic Functionality
///
/// `Migration` facilitates migrating account data from an old schema (`From`) to a new
/// schema (`To`). During deserialization, the account must be in the `From` format -
/// accounts already in the `To` format will be rejected with an error.
///
/// The migrated data is stored in memory and will be serialized to the account when the
/// instruction exits. On exit, the account must be in the migrated state or an error will
/// be returned.
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
/// # Usage Patterns
///
/// There are multiple ways to work with Migration accounts:
///
/// ## 1. Explicit Migration with `migrate()`
///
/// ```ignore
/// ctx.accounts.my_account.migrate(AccountV2 {
///     data: ctx.accounts.my_account.data,
///     new_field: 42,
/// })?;
/// ```
///
/// ## 2. Direct Field Access via Deref (before migration)
///
/// ```ignore
/// // Access old account fields directly
/// let old_value = ctx.accounts.my_account.data;
/// let old_timestamp = ctx.accounts.my_account.timestamp;
///
/// // Then migrate
/// ctx.accounts.my_account.migrate(AccountV2 { ... })?;
/// ```
///
/// ## 3. Idempotent Migration with `into_inner()`
///
/// ```ignore
/// // Migrates if needed, returns reference to new data
/// // Access old fields directly via deref!
/// let migrated = ctx.accounts.my_account.into_inner(AccountV2 {
///     data: ctx.accounts.my_account.data,
///     new_field: ctx.accounts.my_account.data * 2,
/// })?;
///
/// // Use migrated data (safe to call multiple times!)
/// msg!("New field: {}", migrated.new_field);
/// ```
///
/// ## 4. Idempotent Migration with Mutation via `into_inner_mut()`
///
/// ```ignore
/// // Migrates if needed, returns mutable reference
/// let migrated = ctx.accounts.my_account.into_inner_mut(AccountV2 {
///     data: ctx.accounts.my_account.data,
///     new_field: 0,
/// })?;
///
/// // Mutate the new data
/// migrated.new_field = 42;
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
///         // Use idempotent migration with into_inner
///         let migrated = ctx.accounts.my_account.into_inner(AccountV2 {
///             data: ctx.accounts.my_account.data,
///             new_field: ctx.accounts.my_account.data * 2,
///         })?;
///
///         msg!("Migrated! New field: {}", migrated.new_field);
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
            MigrationInner::From(from) => format!("From({:?})", from),
            MigrationInner::To(to) => format!("To({:?})", to),
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
            inner: MigrationInner::From(account),
        }
    }

    /// Returns `true` if the account has been migrated.
    #[inline(always)]
    pub fn is_migrated(&self) -> bool {
        matches!(self.inner, MigrationInner::To(_))
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

        self.inner = MigrationInner::To(new_data);
        Ok(())
    }

    /// Gets a reference to the migrated value, or migrates it with the provided data.
    ///
    /// This method provides flexible access to the migrated state:
    /// - If already migrated, returns a reference to the existing value
    /// - If not migrated, migrates with the provided data, then returns a reference
    ///
    /// # Arguments
    /// * `new_data` - The new `To` value to migrate to (only used if not yet migrated)
    ///
    /// # Example
    /// ```ignore
    /// pub fn process(ctx: Context<MyInstruction>) -> Result<()> {
    ///     // Migrate and get reference in one call
    ///     // Access old fields directly via deref!
    ///     let migrated = ctx.accounts.my_account.into_inner(AccountV2 {
    ///         data: ctx.accounts.my_account.data,
    ///         new_field: 42,
    ///     })?;
    ///
    ///     // Use migrated...
    ///     msg!("Migrated data: {}", migrated.data);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn into_inner(&mut self, new_data: To) -> Result<&To> {
        if !self.is_migrated() {
            self.inner = MigrationInner::To(new_data);
        }

        match &self.inner {
            MigrationInner::To(to) => Ok(to),
            _ => unreachable!(),
        }
    }

    /// Gets a mutable reference to the migrated value, or migrates it with the provided data.
    ///
    /// This method provides flexible mutable access to the migrated state:
    /// - If already migrated, returns a mutable reference to the existing value
    /// - If not migrated, migrates with the provided data, then returns a mutable reference
    ///
    /// # Arguments
    /// * `new_data` - The new `To` value to migrate to (only used if not yet migrated)
    ///
    /// # Example
    /// ```ignore
    /// pub fn process(ctx: Context<MyInstruction>) -> Result<()> {
    ///     // Migrate and get mutable reference in one call
    ///     // Access old fields directly via deref!
    ///     let migrated = ctx.accounts.my_account.into_inner_mut(AccountV2 {
    ///         data: ctx.accounts.my_account.data,
    ///         new_field: 0,
    ///     })?;
    ///
    ///     // Mutate the migrated value
    ///     migrated.new_field = 42;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn into_inner_mut(&mut self, new_data: To) -> Result<&mut To> {
        if !self.is_migrated() {
            self.inner = MigrationInner::To(new_data);
        }

        match &mut self.inner {
            MigrationInner::To(to) => Ok(to),
            _ => unreachable!(),
        }
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
            MigrationInner::From(_) => {
                // Account was not migrated - this is an error
                return Err(ErrorCode::AccountNotMigrated.into());
            }
            MigrationInner::To(to) => {
                // Only persist if the owner is the current program
                let expected_owner = To::owner();
                if &expected_owner != program_id {
                    return Ok(());
                }

                // Serialize the migrated data
                let mut data = self.info.try_borrow_mut_data()?;
                let dst: &mut [u8] = &mut data;
                let mut writer = BpfWriter::new(dst);
                to.try_serialize(&mut writer)?;
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
       match &self.inner {
        MigrationInner::From(from) => from,
        MigrationInner::To(_) =>
            {
                crate::solana_program::msg!("Cannot deref to From: account is already migrated.");
                panic!();
            }
       }
    }
}

// DerefMut to From when account is in Old state
impl<'info, From, To> DerefMut for Migration<'info, From, To>
where
    From: AccountDeserialize,
    To: AccountSerialize,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.inner {
            MigrationInner::From(from) => from,
            MigrationInner::To(_) => {
                crate::solana_program::msg!("Cannot deref_mut to From: account is already migrated.");
                panic!();
            }
        }
    }
}

