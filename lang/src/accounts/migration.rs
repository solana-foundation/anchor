//! Account container for migration from one type to another.

use crate::bpf_writer::BpfWriter;
use crate::error::{Error, ErrorCode};
use crate::solana_program::account_info::AccountInfo;
use crate::solana_program::instruction::AccountMeta;
use crate::solana_program::pubkey::Pubkey;
use crate::solana_program::system_program;
use crate::{
    AccountDeserialize, AccountSerialize, Accounts, AccountsExit, Key, Migrate, Owner, Result,
    ToAccountInfos, ToAccountMetas,
};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt;

/// Internal representation of the migration state.
enum MigrationInner<From, To> {
    /// Account is in old format, will be migrated and serialized on exit
    Old(From),
    /// Account is already in new format, will be serialized on exit
    New(Box<To>),
}

/// Wrapper around [`AccountInfo`](crate::solana_program::account_info::AccountInfo)
/// that handles account schema migrations from old type (`From`) to new type (`To`).
///
/// Migration is lazy - transformation occurs when `as_migrated()` is called,
/// and the migrated data is serialized on exit.
///
/// Checks:
///
/// - `Account.info.owner == From::owner()` or `Account.info.owner == To::owner()` (smart mode)
/// - `!(Account.info.owner == SystemProgram && Account.info.lamports() == 0)`
///
/// # Example
/// ```ignore
/// use anchor_lang::prelude::*;
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
/// impl Migrate<AccountV2> for AccountV1 {
///     fn migrate(&self) -> AccountV2 {
///         AccountV2 {
///             data: self.data,
///             new_field: 0,
///         }
///     }
/// }
///
/// #[derive(Accounts)]
/// pub struct MigrateAccount<'info> {
///     #[account(mut, realloc = 8 + 16, realloc::payer = payer, realloc::zero = false)]
///     pub my_account: Migration<'info, AccountV1, AccountV2>,
///     #[account(mut)]
///     pub payer: Signer<'info>,
///     pub system_program: Program<'info, System>,
/// }
///
/// #[program]
/// pub mod my_program {
///     use super::*;
///     pub fn migrate(ctx: Context<MigrateAccount>) -> Result<()> {
///         let migrated = ctx.accounts.my_account.load();
///         msg!("New field: {}", migrated.new_field);
///         Ok(())
///     }
/// }
/// ```
///
/// # Smart Migration
///
/// By default, `Migration` uses smart detection when both `From` and `To` implement
/// the required traits. It tries to deserialize as `To` first (already migrated),
/// falling back to `From` (needs migration). This allows gradual migrations.
///
/// Use `migrate = "strict"` to reject already-migrated accounts:
/// ```ignore
/// #[account(migrate = "strict")]
/// pub my_account: Migration<'info, AccountV1, AccountV2>,
/// ```
pub struct Migration<'info, From, To>
where
    From: AccountDeserialize + Migrate<To>,
    To: AccountSerialize,
{
    /// Account info reference
    info: &'info AccountInfo<'info>,
    /// Internal migration state - either Old(From) or New(Box<To>)
    inner: RefCell<MigrationInner<From, To>>,
}

impl<'info, From, To> fmt::Debug for Migration<'info, From, To>
where
    From: AccountDeserialize + Clone + Migrate<To> + fmt::Debug,
    To: AccountSerialize + Clone + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner_debug = match &*self.inner.borrow() {
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
    From: AccountDeserialize + Migrate<To>,
    To: AccountSerialize,
{
    fn new(info: &'info AccountInfo<'info>, account: From) -> Self {
        Self {
            info,
            inner: RefCell::new(MigrationInner::Old(account)),
        }
    }

    fn new_already_migrated(info: &'info AccountInfo<'info>, account: To) -> Self {
        Self {
            info,
            inner: RefCell::new(MigrationInner::New(Box::new(account))),
        }
    }


    /// Reloads the account data, performing migration if needed.
    pub fn load(&self) -> std::cell::Ref<To> {
        // Perform migration if still in Old state
        {
            let inner = self.inner.borrow();
            if matches!(*inner, MigrationInner::Old(_)) {
                drop(inner);
                let mut inner_mut = self.inner.borrow_mut();
                if let MigrationInner::Old(from) = &*inner_mut {
                    let migrated = from.migrate();
                    *inner_mut = MigrationInner::New(Box::new(migrated));
                }
            }
        }

        // Return reference to the New variant
        std::cell::Ref::map(self.inner.borrow(), |i| match i {
            MigrationInner::New(to) => to.as_ref(),
            _ => unreachable!("Migration should have been performed"),
        })
    }

    /// Reloads the account data mutably, performing migration if needed.
    pub fn load_mut(&self) -> std::cell::RefMut<To> {
        #[cfg(feature = "anchor-debug")]
        if !self.info.is_writable {
            crate::solana_program::msg!("Migration account is not mutable");
            panic!();
        }

        // Perform migration if still in Old state
        {
            let inner = self.inner.borrow();
            if matches!(*inner, MigrationInner::Old(_)) {
                drop(inner);
                let mut inner_mut = self.inner.borrow_mut();
                if let MigrationInner::Old(from) = &*inner_mut {
                    let migrated = from.migrate();
                    *inner_mut = MigrationInner::New(Box::new(migrated));
                }
            }
        }

        // Return mutable reference to the New variant
        std::cell::RefMut::map(self.inner.borrow_mut(), |i| match i {
            MigrationInner::New(to) => to.as_mut(),
            _ => unreachable!("Migration should have been performed"),
        })
    }

    pub fn into_inner(self) -> To {
        match self.inner.into_inner() {
            MigrationInner::Old(from) => from.migrate(),
            MigrationInner::New(to) => *to,
        }
    }

    /// Returns `true` if the account was already in the new format when deserialized.
    /// 
    /// This method is used to check if the account was already in the new format when deserialized in the
    /// migration constraint. This is why it is hidden.
    #[doc(hidden)]
    pub fn is_already_migrated(&self) -> bool {
        matches!(*self.inner.borrow(), MigrationInner::New(_))
    }
}

impl<'info, From, To> Migration<'info, From, To>
where
    From: AccountDeserialize + Migrate<To> + Owner,
    To: AccountSerialize,
{
    /// Deserializes the given `info` into a `Migration`.
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

impl<'info, From, To> Migration<'info, From, To>
where
    From: AccountDeserialize + Migrate<To> + Owner,
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

        // Try to deserialize as To (already migrated)
        let data_borrowed = self.info.try_borrow_data()?;
        let mut data: &[u8] = &data_borrowed;
        if self.info.owner == &To::owner() {
            if let Ok(already_migrated) = To::try_deserialize(&mut data) {
                // Account is already migrated!
                drop(data_borrowed);
                *self.inner.borrow_mut() = MigrationInner::New(Box::new(already_migrated));
                return Ok(());
            }
        }
        drop(data_borrowed);

        // Fall back to deserializing as From
        let mut data: &[u8] = &self.info.try_borrow_data()?;
        if self.info.owner != &From::owner() {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*self.info.owner, From::owner())));
        }
        let from = From::try_deserialize(&mut data)?;
        *self.inner.borrow_mut() = MigrationInner::Old(from);
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
    From: AccountDeserialize + Migrate<To> + Owner,
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
    From: AccountDeserialize + Migrate<To> + Owner,
    To: AccountSerialize,
{
    fn exit(&self, program_id: &Pubkey) -> Result<()> {
        // Only persist if the owner is the current program and the account is not closed
        let expected_owner = From::owner();
        if &expected_owner != program_id || crate::common::is_closed(self.info) {
            return Ok(());
        }

        // Perform migration and serialize
        let mut data = self.info.try_borrow_mut_data()?;
        let dst: &mut [u8] = &mut data;
        let mut writer = BpfWriter::new(dst);

        // Serialize based on the current inner state
        let inner = self.inner.borrow();
        match &*inner {
            MigrationInner::Old(from) => {
                // Perform migration and serialize the result
                let to = from.migrate();
                to.try_serialize(&mut writer)?;
            }
            MigrationInner::New(to) => {
                // Already migrated, serialize the cached value
                to.as_ref().try_serialize(&mut writer)?;
            }
        }

        Ok(())
    }
}

impl<From, To> ToAccountMetas for Migration<'_, From, To>
where
    From: AccountDeserialize + Migrate<To>,
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
    From: AccountDeserialize + Migrate<To>,
    To: AccountSerialize,
{
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.info.clone()]
    }
}

impl<'info, From, To> AsRef<AccountInfo<'info>> for Migration<'info, From, To>
where
    From: AccountDeserialize + Migrate<To>,
    To: AccountSerialize,
{
    fn as_ref(&self) -> &AccountInfo<'info> {
        self.info
    }
}


impl<From, To> Key for Migration<'_, From, To>
where
    From: AccountDeserialize + Migrate<To>,
    To: AccountSerialize,
{
    fn key(&self) -> Pubkey {
        *self.info.key
    }
}
