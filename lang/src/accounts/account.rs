//! Generic account container driven by check policies.

use {
    crate::{
        bpf_writer::BpfWriter,
        error::{Error, ErrorCode},
        pinocchio_runtime::{
            account_view::AccountView,
            bpf_loader_upgradeable::{self, UpgradeableLoaderState},
            instruction::AccountMeta,
            pubkey::Pubkey,
            system_program,
        },
        AccountDeserialize, AccountSerialize, Accounts, AccountsClose, AccountsExit, Id, Key,
        Owner, Result, ToAccountMetas, ToAccountView, ToAccountViews,
    },
    std::{
        collections::BTreeSet,
        fmt,
        marker::PhantomData,
        ops::{Deref, DerefMut},
    },
};

/// Wrapper around [`AccountView`](crate::pinocchio_runtime::account_view::AccountView)
/// that verifies program ownership and deserializes underlying data into a Rust type.
///
/// # Table of Contents
/// - [Basic Functionality](#basic-functionality)
/// - [Using Account with non-anchor types](#using-account-with-non-anchor-types)
/// - [Out of the box wrapper types](#out-of-the-box-wrapper-types)
///
/// # Basic Functionality
///
/// Account checks that `Account.info.owner == T::owner()`.
/// This means that the data type that Accounts wraps around (`=T`) needs to
/// implement the [Owner trait](crate::Owner).
/// The `#[account]` attribute implements the Owner trait for
/// a struct using the `crate::ID` declared by [`declare_id`](crate::declare_id)
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
///     pub fn set_data(ctx: Context<SetData>, data: u64) -> Result<()> {
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
///
/// instead of using `#[account]`. You only have to implement a fraction of the
/// functions `#[account]` generates. See the example below for the code you have
/// to write.
///
/// The mint wrapper type that Anchor provides out of the box for the token program ([source](https://github.com/coral-xyz/anchor/blob/master/spl/src/token.rs))
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
///     fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
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
///
/// ## Out of the box wrapper types
///
/// ### Accessing BPFUpgradeableLoader Data
///
/// Anchor provides wrapper types to access data stored in programs owned by the BPFUpgradeableLoader
/// such as the upgrade authority. If you're interested in the data of a program account, you can use
/// ```ignore
/// Account<'info, BpfUpgradeableLoaderState>
/// ```
/// and then match on its contents inside your instruction function.
///
/// Alternatively, you can use
/// ```ignore
/// Account<'info, ProgramData>
/// ```
/// to let anchor do the matching for you and return the ProgramData variant of BpfUpgradeableLoaderState.
///
/// # Example
/// ```ignore
/// use anchor_lang::prelude::*;
/// use crate::program::MyProgram;
///
/// declare_id!("Cum9tTyj5HwcEiAmhgaS7Bbj4UczCwsucrCkxRECzM4e");
///
/// #[program]
/// pub mod my_program {
///     use super::*;
///
///     pub fn set_initial_admin(
///         ctx: Context<SetInitialAdmin>,
///         admin_key: Pubkey
///     ) -> Result<()> {
///         ctx.accounts.admin_settings.admin_key = admin_key;
///         Ok(())
///     }
///
///     pub fn set_admin(...){...}
///
///     pub fn set_settings(...){...}
/// }
///
/// #[account]
/// #[derive(Default, Debug)]
/// pub struct AdminSettings {
///     admin_key: Pubkey
/// }
///
/// #[derive(Accounts)]
/// pub struct SetInitialAdmin<'info> {
///     #[account(init, payer = authority, seeds = [b"admin"], bump)]
///     pub admin_settings: Account<'info, AdminSettings>,
///     #[account(mut)]
///     pub authority: Signer<'info>,
///     #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
///     pub program: Program<'info, MyProgram>,
///     #[account(constraint = program_data.upgrade_authority_address == Some(authority.key()))]
///     pub program_data: Account<'info, ProgramData>,
///     pub system_program: Program<'info, System>,
/// }
/// ```
///
/// This example solves a problem you may face if your program has admin settings: How do you set the
/// admin key for restricted functionality after deployment? Setting the admin key itself should
/// be a restricted action but how do you restrict it without having set an admin key?
/// You're stuck in a loop.
/// One solution is to use the upgrade authority of the program as the initial
/// (or permanent) admin key.
///
/// ### SPL Types
///
/// Anchor provides wrapper types to access accounts owned by the token program. Use
/// ```ignore
/// use anchor_spl::token::TokenAccount;
///
/// #[derive(Accounts)]
/// pub struct Example {
///     pub my_acc: Account<'info, TokenAccount>
/// }
/// ```
/// to access token accounts and
/// ```ignore
/// use anchor_spl::token::Mint;
///
/// #[derive(Accounts)]
/// pub struct Example {
///     pub my_acc: Account<'info, Mint>
/// }
/// ```
/// to access mint accounts.
pub trait AccountChecks {
    type Target: Clone;
    fn check(info: &AccountView) -> Result<()>;
    fn load(info: &AccountView) -> Result<Self::Target>;
    fn reload(_current: &Self::Target, info: &AccountView) -> Result<Self::Target> {
        Self::load(info)
    }
    fn persist(_value: &Self::Target, _info: &AccountView, _program_id: &Pubkey) -> Result<()> {
        Ok(())
    }
}

pub trait AccountData: AccountChecks {
    fn as_target_ref(value: &Self::Target) -> &Self;
    fn as_target_mut(value: &mut Self::Target) -> &mut Self;
    fn set_target(value: &mut Self::Target, next: Self);
    fn into_target(value: Self::Target) -> Self;
}

#[derive(Clone, Debug)]
pub struct Wallet;

#[derive(Clone, Debug)]
pub struct System;

#[derive(Clone, Debug)]
pub struct Program<T>(PhantomData<T>);

#[derive(Clone, Debug)]
pub struct AnyProgram;

#[derive(Clone)]
pub struct Account<'info, T: AccountChecks = ()> {
    account: T::Target,
    info: AccountView,
    _marker: PhantomData<(&'info (), T)>,
}

impl<T> fmt::Debug for Account<'_, T>
where
    T: AccountChecks,
    T::Target: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_name("Account", f)
    }
}

impl<T> Account<'_, T>
where
    T: AccountChecks,
    T::Target: fmt::Debug,
{
    pub(crate) fn fmt_with_name(&self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(name)
            .field("account", &self.account)
            .field("info", &self.info)
            .finish()
    }
}

impl<'a, T: AccountChecks> Account<'a, T> {
    pub(crate) fn new(info: AccountView, account: T::Target) -> Account<'a, T> {
        Self {
            info,
            account,
            _marker: PhantomData,
        }
    }

    pub(crate) fn exit_with_expected_owner(
        &self,
        _expected_owner: &Pubkey,
        program_id: &Pubkey,
    ) -> Result<()> {
        T::persist(&self.account, &self.info, program_id)
    }

    pub fn view(&self) -> &AccountView {
        &self.info
    }

    pub fn lamports(&self) -> u64 {
        self.info.lamports()
    }

    pub fn is_writable(&self) -> bool {
        self.info.is_writable()
    }
}

impl<'a, T> Account<'a, T>
where
    T: AccountData,
{
    pub fn into_inner(self) -> T {
        T::into_target(self.account)
    }

    pub fn set_inner(&mut self, inner: T) {
        T::set_target(&mut self.account, inner);
    }
}

impl<'a, T: AccountChecks> Account<'a, T> {
    pub fn reload(&mut self) -> Result<()> {
        T::check(&self.info)?;
        self.account = T::reload(&self.account, &self.info)?;
        Ok(())
    }

    #[inline(never)]
    pub fn try_from(info: AccountView) -> Result<Account<'a, T>> {
        T::check(&info)?;
        Ok(Account::new(info, T::load(&info)?))
    }

    #[inline(never)]
    pub fn try_from_unchecked(info: AccountView) -> Result<Account<'a, T>> {
        Self::try_from(info)
    }
}

impl<'info, B, T: AccountChecks> Accounts<'info, B> for Account<'info, T> {
    #[inline(never)]
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &'info [AccountView],
        _ix_data: &[u8],
        _bumps: &mut B,
        _reallocs: &mut BTreeSet<Pubkey>,
    ) -> Result<Self> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = accounts[0];
        *accounts = &accounts[1..];
        Account::try_from(account)
    }
}

impl<'info, T: AccountChecks> AccountsExit<'info> for Account<'info, T> {
    fn exit(&self, program_id: &Pubkey) -> Result<()> {
        T::persist(&self.account, &self.info, program_id)
    }
}

impl<'info, T: AccountChecks> AccountsClose<'info> for Account<'info, T> {
    fn close(&self, sol_destination: AccountView) -> Result<()> {
        crate::common::close(self.to_account_view(), sol_destination)
    }
}

impl<T: AccountChecks> ToAccountMetas for Account<'_, T> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta<'_>> {
        let is_signer = is_signer.unwrap_or(self.info.is_signer());
        let meta = match (self.info.is_writable(), is_signer) {
            (false, false) => AccountMeta::readonly(self.info.address()),
            (false, true) => AccountMeta::readonly_signer(self.info.address()),
            (true, false) => AccountMeta::writable(self.info.address()),
            (true, true) => AccountMeta::writable_signer(self.info.address()),
        };
        vec![meta]
    }
}

impl<'info, T: AccountChecks> ToAccountViews for Account<'info, T> {
    fn to_account_views(&self) -> Vec<AccountView> {
        vec![self.info]
    }
}

impl<'info, T: AccountChecks> AsRef<AccountView> for Account<'info, T> {
    fn as_ref(&self) -> &AccountView {
        &self.info
    }
}

impl<T> AsRef<T> for Account<'_, T>
where
    T: AccountData,
{
    fn as_ref(&self) -> &T {
        T::as_target_ref(&self.account)
    }
}

impl<T> Deref for Account<'_, T>
where
    T: AccountData,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        T::as_target_ref(&self.account)
    }
}

impl<T> DerefMut for Account<'_, T>
where
    T: AccountData,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        #[cfg(feature = "anchor-debug")]
        if !self.info.is_writable() {
            crate::pinocchio_runtime::msg!("The given Account is not mutable");
            panic!();
        }
        T::as_target_mut(&mut self.account)
    }
}

impl<T: AccountChecks> Key for Account<'_, T> {
    fn key(&self) -> Pubkey {
        *self.info.address()
    }
}

impl AccountChecks for () {
    type Target = ();
    fn check(_info: &AccountView) -> Result<()> {
        Ok(())
    }
    fn load(_info: &AccountView) -> Result<Self::Target> {
        Ok(())
    }
}

impl AccountChecks for Wallet {
    type Target = ();
    fn check(info: &AccountView) -> Result<()> {
        if !info.is_signer() {
            return Err(ErrorCode::AccountNotSigner.into());
        }
        Ok(())
    }
    fn load(_info: &AccountView) -> Result<Self::Target> {
        Ok(())
    }
}

impl AccountChecks for System {
    type Target = ();
    fn check(info: &AccountView) -> Result<()> {
        if !info.owned_by(&system_program::ID) {
            return Err(ErrorCode::AccountNotSystemOwned.into());
        }
        Ok(())
    }
    fn load(_info: &AccountView) -> Result<Self::Target> {
        Ok(())
    }
}

impl<P: Id> AccountChecks for Program<P> {
    type Target = ();
    fn check(info: &AccountView) -> Result<()> {
        if info.address() != &P::id() {
            return Err(
                Error::from(ErrorCode::InvalidProgramId).with_pubkeys((info.key(), P::id()))
            );
        }
        if !info.executable() {
            return Err(ErrorCode::InvalidProgramExecutable.into());
        }
        Ok(())
    }
    fn load(_info: &AccountView) -> Result<Self::Target> {
        Ok(())
    }
}

impl AccountChecks for Program<AnyProgram> {
    type Target = ();
    fn check(info: &AccountView) -> Result<()> {
        if !info.executable() {
            return Err(ErrorCode::InvalidProgramExecutable.into());
        }
        Ok(())
    }
    fn load(_info: &AccountView) -> Result<Self::Target> {
        Ok(())
    }
}

impl<T> AccountChecks for T
where
    T: AccountSerialize + AccountDeserialize + Owner + Clone,
{
    type Target = T;
    fn check(info: &AccountView) -> Result<()> {
        if info.owned_by(&system_program::ID) && info.lamports() == 0 {
            return Err(ErrorCode::AccountNotInitialized.into());
        }
        if !info.owned_by(&T::owner()) {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*info.owner(), T::owner())));
        }
        Ok(())
    }
    fn load(info: &AccountView) -> Result<Self::Target> {
        let data = info.try_borrow()?;
        let mut data: &[u8] = &data;
        T::try_deserialize(&mut data)
    }
    fn reload(_current: &Self::Target, info: &AccountView) -> Result<Self::Target> {
        let data = info.try_borrow()?;
        let mut data: &[u8] = &data;
        T::try_deserialize(&mut data)
    }
    fn persist(value: &Self::Target, info: &AccountView, program_id: &Pubkey) -> Result<()> {
        if &T::owner() == program_id && !crate::common::is_closed(info) {
            let mut info = *info;
            let mut data = info.try_borrow_mut()?;
            let dst: &mut [u8] = &mut data;
            let mut writer = BpfWriter::new(dst);
            value.try_serialize(&mut writer)?;
        }
        Ok(())
    }
}

impl<T> AccountData for T
where
    T: AccountSerialize + AccountDeserialize + Owner + Clone,
{
    fn as_target_ref(value: &Self::Target) -> &Self {
        value
    }
    fn as_target_mut(value: &mut Self::Target) -> &mut Self {
        value
    }
    fn set_target(value: &mut Self::Target, next: Self) {
        *value = next;
    }
    fn into_target(value: Self::Target) -> Self {
        value
    }
}

impl<'a, P> Account<'a, Program<P>>
where
    Program<P>: AccountChecks,
{
    pub fn programdata_address(&self) -> Result<Option<Pubkey>> {
        if self.info.owned_by(&bpf_loader_upgradeable::ID) {
            let mut data: &[u8] = &self.info.try_borrow()?;
            let upgradable_loader_state =
                UpgradeableLoaderState::try_deserialize_unchecked(&mut data)?;
            match upgradable_loader_state {
                UpgradeableLoaderState::Program {
                    programdata_address,
                } => Ok(Some(programdata_address)),
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }
}
