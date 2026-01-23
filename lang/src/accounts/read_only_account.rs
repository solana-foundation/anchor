//! Read-only account container that checks ownership on deserialization.
//!
//! This is used internally by the `#[derive(Accounts)]` macro for accounts that 
//! are not marked with `#[account(mut)]`.

use crate::error::{Error, ErrorCode};
use crate::solana_program::account_info::AccountInfo;
use crate::solana_program::instruction::AccountMeta;
use crate::solana_program::pubkey::Pubkey;
use crate::solana_program::system_program;
use crate::{
    AccountDeserialize, AccountSerialize, Accounts, AccountsClose, AccountsExit, Key, Owner,
    Result, ToAccountInfo, ToAccountInfos, ToAccountMetas,
};
use std::collections::BTreeSet;
use std::fmt;
use std::ops::Deref;

/// Read-only wrapper around [`AccountInfo`](crate::solana_program::account_info::AccountInfo)
/// that verifies program ownership and deserializes underlying data into a Rust type.
///
/// # When is this used?
///
/// The `#[derive(Accounts)]` macro automatically uses `ReadOnlyAccount` for accounts
/// that are **not** marked with `#[account(mut)]`. This provides compile-time safety
/// against accidentally mutating accounts that won't have their changes persisted.
///
/// # Example
///
/// ```ignore
/// use anchor_lang::prelude::*;
///
/// #[derive(Accounts)]
/// pub struct ReadData<'info> {
///     pub data_account: Account<'info, MyData>,
/// }
///
/// pub fn read_data(ctx: Context<ReadData>) -> Result<()> {
///     let value = ctx.accounts.data_account.value;
///     
///     // This would cause a compile-time error:
///     ctx.accounts.data_account.value = 42;
///     //                        ^^^^^ cannot assign to data in a `&` reference
///     
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct ReadOnlyAccount<'info, T: AccountSerialize + AccountDeserialize + Clone> {
    account: T,
    info: &'info AccountInfo<'info>,
}

impl<T: AccountSerialize + AccountDeserialize + Clone + fmt::Debug> fmt::Debug
    for ReadOnlyAccount<'_, T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadOnlyAccount")
            .field("account", &self.account)
            .field("info", &self.info)
            .finish()
    }
}

impl<'a, T: AccountSerialize + AccountDeserialize + Clone> ReadOnlyAccount<'a, T> {
    pub(crate) fn new(info: &'a AccountInfo<'a>, account: T) -> ReadOnlyAccount<'a, T> {
        Self { info, account }
    }

    /// Consumes the wrapper and returns the inner account data.
    pub fn into_inner(self) -> T {
        self.account
    }
}

impl<'a, T: AccountSerialize + AccountDeserialize + Owner + Clone> ReadOnlyAccount<'a, T> {
    /// Reloads the account from storage.
    pub fn reload(&mut self) -> Result<()> {
        if self.info.owner != &T::owner() {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*self.info.owner, T::owner())));
        }

        let mut data: &[u8] = &self.info.try_borrow_data()?;
        self.account = T::try_deserialize(&mut data)?;
        Ok(())
    }

    /// Deserializes the given `info` into a `ReadOnlyAccount`.
    #[inline(never)]
    pub fn try_from(info: &'a AccountInfo<'a>) -> Result<ReadOnlyAccount<'a, T>> {
        if info.owner == &system_program::ID && info.lamports() == 0 {
            return Err(ErrorCode::AccountNotInitialized.into());
        }
        if info.owner != &T::owner() {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*info.owner, T::owner())));
        }
        let mut data: &[u8] = &info.try_borrow_data()?;
        Ok(ReadOnlyAccount::new(info, T::try_deserialize(&mut data)?))
    }
}

impl<'info, B, T: AccountSerialize + AccountDeserialize + Owner + Clone> Accounts<'info, B>
    for ReadOnlyAccount<'info, T>
where
    T: AccountSerialize + AccountDeserialize + Owner + Clone,
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
        ReadOnlyAccount::try_from(account)
    }
}

/// ReadOnlyAccount's exit is a no-op since no changes need to be persisted.
impl<'info, T: AccountSerialize + AccountDeserialize + Owner + Clone> AccountsExit<'info>
    for ReadOnlyAccount<'info, T>
{
    fn exit(&self, _program_id: &Pubkey) -> Result<()> {
        // No-op: ReadOnlyAccount never persists changes
        Ok(())
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> AccountsClose<'info>
    for ReadOnlyAccount<'info, T>
{
    fn close(&self, sol_destination: AccountInfo<'info>) -> Result<()> {
        crate::common::close(self.to_account_info(), sol_destination)
    }
}

impl<T: AccountSerialize + AccountDeserialize + Clone> ToAccountMetas for ReadOnlyAccount<'_, T> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.info.is_signer);
        // ReadOnlyAccount is always non-writable
        vec![AccountMeta::new_readonly(*self.info.key, is_signer)]
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> ToAccountInfos<'info>
    for ReadOnlyAccount<'info, T>
{
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.info.clone()]
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> AsRef<AccountInfo<'info>>
    for ReadOnlyAccount<'info, T>
{
    fn as_ref(&self) -> &AccountInfo<'info> {
        self.info
    }
}

impl<T: AccountSerialize + AccountDeserialize + Clone> AsRef<T> for ReadOnlyAccount<'_, T> {
    fn as_ref(&self) -> &T {
        &self.account
    }
}

impl<T: AccountSerialize + AccountDeserialize + Clone> Deref for ReadOnlyAccount<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.account
    }
}

impl<T: AccountSerialize + AccountDeserialize + Clone> Key for ReadOnlyAccount<'_, T> {
    fn key(&self) -> Pubkey {
        *self.info.key
    }
}
