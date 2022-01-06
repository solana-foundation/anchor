//! Account container that checks ownership on deserialization.

use crate::error::ErrorCode;
use crate::*;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::fmt;
use std::ops::{Deref, DerefMut};

/// Wrapper around [`AccountInfo`](crate::solana_program::account_info::AccountInfo)
/// that verifies program ownership and deserializes underlying data into a Rust type.
///
/// Account checks that `Account.info.owner == T::owner()`.
/// This means that the data type that Accounts wraps around (`=T`) needs to
/// implement the [Owner trait](crate::Owner).
/// The `#[account]` attribute implements the Owner trait for
/// a struct using the `crate::ID` declared by [`declareId`](crate::declare_id)
/// in the same program. It follows that Account can also be used
/// with a `T` that comes from a different program.
///
/// Checks:
///
/// - `Account.info.owner == T::owner()`
/// - `!(Account.info.owner == SystemProgram && Account.info.lamports() == 0)`
///
/// # Example
/// ```ignore
/// use anchor_lang::prelude::*;
/// use other_program::Auth;
///
/// declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
///
/// #[program]
/// mod hello_anchor {
///     use super::*;
///     pub fn set_data(ctx: Context<SetData>, data: u64) -> ProgramResult {
///         if (*ctx.accounts.auth_account).authorized {
///             (*ctx.accounts.my_account).data = data;
///         }
///         Ok(())
///     }
/// }
///
/// #[account]
/// #[derive(Default)]
/// pub struct MyData {
///     pub data: u64
/// }
///
/// #[derive(Accounts)]
/// pub struct SetData<'info> {
///     #[account(mut)]
///     pub my_account: Account<'info, MyData> // checks that my_account.info.owner == Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS
///     pub auth_account: Account<'info, Auth> // checks that auth_account.info.owner == FEZGUxNhZWpYPj9MJCrZJvUo1iF9ys34UHx52y4SzVW9
/// }
///
/// // In a different program
///
/// ...
/// declare_id!("FEZGUxNhZWpYPj9MJCrZJvUo1iF9ys34UHx52y4SzVW9");
/// #[account]
/// #[derive(Default)]
/// pub struct Auth {
///     pub authorized: bool
/// }
/// ...
/// ```
///
/// # Using Account with non-anchor programs
///
/// Account can also be used with non-anchor programs. The data types from
/// those programs are not annotated with `#[account]` so you have to
/// - create a wrapper type around the structs you want to wrap with Account
/// - implement the functions required by Account yourself
/// instead of using `#[account]`. You only have to implement a fraction of the
/// functions `#[account]` generates. See the example below for the code you have
/// to write.
///
/// The mint wrapper type Anchor provides out of the box for the token program ([source](https://github.com/project-serum/anchor/blob/master/spl/src/token.rs))
/// ```ignore
/// #[derive(Clone)]
/// pub struct Mint(spl_token::state::Mint);
///
/// // This is necessary so we can use "anchor_spl::token::Mint::LEN"
/// // because rust does not resolve "anchor_spl::token::Mint::LEN" to
/// // "spl_token::state::Mint::LEN" automatically
/// impl Mint {
///     pub const LEN: usize = spl_token::state::Mint::LEN;
/// }
///
/// // You don't have to implement the "try_deserialize" function
/// // from this trait. It delegates to
/// // "try_deserialize_unchecked" by default which is what we want here
/// // because non-anchor accounts don't have a discriminator to check
/// impl anchor_lang::AccountDeserialize for Mint {
///     fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError> {
///         spl_token::state::Mint::unpack(buf).map(Mint)
///     }
/// }
/// // AccountSerialize defaults to a no-op which is what we want here
/// // because it's a foreign program, so our program does not
/// // have permission to write to the foreign program's accounts anyway
/// impl anchor_lang::AccountSerialize for Mint {}
///
/// impl anchor_lang::Owner for Mint {
///     fn owner() -> Pubkey {
///         // pub use spl_token::ID is used at the top of the file
///         ID
///     }
/// }
///
/// // Implement the "std::ops::Deref" trait for better user experience
/// impl Deref for Mint {
///     type Target = spl_token::state::Mint;
///
///     fn deref(&self) -> &Self::Target {
///         &self.0
///     }
/// }
/// ```
#[derive(Clone)]
pub struct Account<'info, T: AccountSerialize + AccountDeserialize + Owner + Clone> {
    account: T,
    info: AccountInfo<'info>,
}

impl<'info, T: AccountSerialize + AccountDeserialize + Owner + Clone + fmt::Debug> fmt::Debug
    for Account<'info, T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Account")
            .field("account", &self.account)
            .field("info", &self.info)
            .finish()
    }
}

impl<'a, T: AccountSerialize + AccountDeserialize + Owner + Clone> Account<'a, T> {
    fn new(info: AccountInfo<'a>, account: T) -> Account<'a, T> {
        Self { info, account }
    }

    /// Deserializes the given `info` into a `Account`.
    #[inline(never)]
    pub fn try_from(info: &AccountInfo<'a>) -> Result<Account<'a, T>, ProgramError> {
        if info.owner == &system_program::ID && info.lamports() == 0 {
            return Err(ErrorCode::AccountNotInitialized.into());
        }
        if info.owner != &T::owner() {
            return Err(ErrorCode::AccountOwnedByWrongProgram.into());
        }
        let mut data: &[u8] = &info.try_borrow_data()?;
        Ok(Account::new(info.clone(), T::try_deserialize(&mut data)?))
    }

    /// Deserializes the given `info` into a `Account` without checking
    /// the account discriminator. Be careful when using this and avoid it if
    /// possible.
    #[inline(never)]
    pub fn try_from_unchecked(info: &AccountInfo<'a>) -> Result<Account<'a, T>, ProgramError> {
        if info.owner == &system_program::ID && info.lamports() == 0 {
            return Err(ErrorCode::AccountNotInitialized.into());
        }
        if info.owner != &T::owner() {
            return Err(ErrorCode::AccountOwnedByWrongProgram.into());
        }
        let mut data: &[u8] = &info.try_borrow_data()?;
        Ok(Account::new(
            info.clone(),
            T::try_deserialize_unchecked(&mut data)?,
        ))
    }

    /// Reloads the account from storage. This is useful, for example, when
    /// observing side effects after CPI.
    pub fn reload(&mut self) -> ProgramResult {
        let mut data: &[u8] = &self.info.try_borrow_data()?;
        self.account = T::try_deserialize(&mut data)?;
        Ok(())
    }

    pub fn into_inner(self) -> T {
        self.account
    }

    pub fn set_inner(&mut self, inner: T) {
        self.account = inner;
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Owner + Clone> Accounts<'info>
    for Account<'info, T>
where
    T: AccountSerialize + AccountDeserialize + Owner + Clone,
{
    #[inline(never)]
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
        _ix_data: &[u8],
    ) -> Result<Self, ProgramError> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        Account::try_from(account)
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Owner + Clone> AccountsExit<'info>
    for Account<'info, T>
{
    fn exit(&self, program_id: &Pubkey) -> ProgramResult {
        // Only persist if the owner is the current program.
        if &T::owner() == program_id {
            let info = self.to_account_info();
            let mut data = info.try_borrow_mut_data()?;
            let dst: &mut [u8] = &mut data;
            let mut cursor = std::io::Cursor::new(dst);
            self.account.try_serialize(&mut cursor)?;
        }
        Ok(())
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Owner + Clone> AccountsClose<'info>
    for Account<'info, T>
{
    fn close(&self, sol_destination: AccountInfo<'info>) -> ProgramResult {
        crate::common::close(self.to_account_info(), sol_destination)
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Owner + Clone> ToAccountMetas
    for Account<'info, T>
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

impl<'info, T: AccountSerialize + AccountDeserialize + Owner + Clone> ToAccountInfos<'info>
    for Account<'info, T>
{
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.info.clone()]
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Owner + Clone> ToAccountInfo<'info>
    for Account<'info, T>
{
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.info.clone()
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Owner + Clone> AsRef<AccountInfo<'info>>
    for Account<'info, T>
{
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.info
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Owner + Clone> AsRef<T>
    for Account<'info, T>
{
    fn as_ref(&self) -> &T {
        &self.account
    }
}

impl<'a, T: AccountSerialize + AccountDeserialize + Owner + Clone> Deref for Account<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &(*self).account
    }
}

impl<'a, T: AccountSerialize + AccountDeserialize + Owner + Clone> DerefMut for Account<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        #[cfg(feature = "anchor-debug")]
        if !self.info.is_writable {
            solana_program::msg!("The given Account is not mutable");
            panic!();
        }
        &mut self.account
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Owner + Clone> Key for Account<'info, T> {
    fn key(&self) -> Pubkey {
        *self.info.key
    }
}
