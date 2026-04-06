//! [`Account`] wraps an [`AccountView`] and pairs it with a payload chosen by `T: AccountChecks`.
//! Typed account data uses the blanket impl for `AccountSerialize + AccountDeserialize + Owner + Clone`;
//! signers, the system program, and executable programs use markers such as [`Wallet`], [`System`], and [`Program`].

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

mod private {
    /// [`super::AccountChecks`] is sealed: implementations live only in this module.
    pub trait Sealed {}
}

/// Describes how to validate an [`AccountView`] and produce the value stored in [`Account<T>`].
///
/// Markers ([`Wallet`], [`System`], [`Program`], [`AnyProgram`], `()`) cover signers, system-owned
/// accounts, and programs. Typed account data uses the blanket impl for
/// `AccountSerialize + AccountDeserialize + Owner + Clone` (for example types from `#[account]`).
pub trait AccountChecks: private::Sealed {
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

/// Typed account: an [`AccountView`] plus the loaded or marker payload for `T: AccountChecks`.
#[derive(Clone)]
pub struct Account<T: AccountChecks = ()> {
    account: T::Target,
    info: AccountView,
    _marker: PhantomData<T>,
}

impl<T> fmt::Debug for Account<T>
where
    T: AccountChecks,
    T::Target: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_name("Account", f)
    }
}

impl<T> Account<T>
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

impl<T: AccountChecks> Account<T> {
    pub(crate) fn new(info: AccountView, account: T::Target) -> Account<T> {
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

    pub fn key(&self) -> Pubkey {
        *self.info.address()
    }

    pub fn lamports(&self) -> u64 {
        self.info.lamports()
    }

    pub fn is_writable(&self) -> bool {
        self.info.is_writable()
    }
}

impl<T> Account<T>
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

impl<T: AccountChecks> Account<T> {
    pub fn reload(&mut self) -> Result<()> {
        T::check(&self.info)?;
        self.account = T::reload(&self.account, &self.info)?;
        Ok(())
    }

    #[inline(never)]
    pub fn try_from(info: AccountView) -> Result<Account<T>> {
        T::check(&info)?;
        Ok(Account::new(info, T::load(&info)?))
    }

    #[inline(never)]
    pub fn try_from_unchecked(info: AccountView) -> Result<Account<T>> {
        Self::try_from(info)
    }
}

impl<'info, B, T: AccountChecks> Accounts<'info, B> for Account<T> {
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

impl<'info, T: AccountChecks> AccountsExit<'info> for Account<T> {
    fn exit(&self, program_id: &Pubkey) -> Result<()> {
        T::persist(&self.account, &self.info, program_id)
    }
}

impl<T: AccountChecks> AccountsClose for Account<T> {
    fn close(&self, sol_destination: AccountView) -> Result<()> {
        crate::common::close(self.to_account_view(), sol_destination)
    }
}

impl<T: AccountChecks> ToAccountMetas for Account<T> {
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

impl<T: AccountChecks> ToAccountViews for Account<T> {
    fn to_account_views(&self) -> Vec<AccountView> {
        vec![self.info]
    }
}

impl<T: AccountChecks> AsRef<AccountView> for Account<T> {
    fn as_ref(&self) -> &AccountView {
        &self.info
    }
}

impl<T> AsRef<T> for Account<T>
where
    T: AccountData,
{
    fn as_ref(&self) -> &T {
        T::as_target_ref(&self.account)
    }
}

impl<T> Deref for Account<T>
where
    T: AccountData,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        T::as_target_ref(&self.account)
    }
}

impl<T> DerefMut for Account<T>
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

impl<T: AccountChecks> Key for Account<T> {
    fn key(&self) -> Pubkey {
        Account::key(self)
    }
}

impl private::Sealed for () {}

impl private::Sealed for Wallet {}

impl private::Sealed for System {}

impl private::Sealed for AnyProgram {}

impl<P> private::Sealed for Program<P> {}

impl<T> private::Sealed for T where T: AccountSerialize + AccountDeserialize + Owner + Clone {}

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

impl<P> Account<Program<P>>
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
