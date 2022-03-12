use crate::*;
use crate::{error::ErrorCode, prelude::Account};
use solana_program::account_info::AccountInfo;
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;
use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

/// Container for any account *not* owned by the current program.
#[derive(Clone)]
#[deprecated(since = "0.15.0", note = "Please use Account instead")]
pub struct CpiAccount<'a, T: AccountDeserialize + Clone> {
    info: AccountInfo<'a>,
    account: Box<T>,
}

#[allow(deprecated)]
impl<'a, T: AccountDeserialize + Clone> CpiAccount<'a, T> {
    fn new(info: AccountInfo<'a>, account: Box<T>) -> CpiAccount<'a, T> {
        Self { info, account }
    }

    /// Deserializes the given `info` into a `CpiAccount`.
    pub fn try_from(info: &AccountInfo<'a>) -> Result<CpiAccount<'a, T>> {
        let mut data: &[u8] = &info.try_borrow_data()?;
        Ok(CpiAccount::new(
            info.clone(),
            Box::new(T::try_deserialize(&mut data)?),
        ))
    }

    pub fn try_from_unchecked(info: &AccountInfo<'a>) -> Result<CpiAccount<'a, T>> {
        Self::try_from(info)
    }

    /// Reloads the account from storage. This is useful, for example, when
    /// observing side effects after CPI.
    pub fn reload(&mut self) -> Result<()> {
        let mut data: &[u8] = &self.info.try_borrow_data()?;
        self.account = Box::new(T::try_deserialize(&mut data)?);
        Ok(())
    }
}

#[allow(deprecated)]
impl<'info, T> Accounts<'info> for CpiAccount<'info, T>
where
    T: AccountDeserialize + Clone,
{
    #[inline(never)]
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
        _ix_data: &[u8],
        _bumps: &mut BTreeMap<String, u8>,
    ) -> Result<Self> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        // No owner check is done here.
        let pa = CpiAccount::try_from(account)?;
        Ok(pa)
    }
}

#[allow(deprecated)]
impl<'info, T: AccountDeserialize + Clone> ToAccountMetas for CpiAccount<'info, T> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.info.is_signer);
        let meta = match self.info.is_writable {
            false => AccountMeta::new_readonly(*self.info.key, is_signer),
            true => AccountMeta::new(*self.info.key, is_signer),
        };
        vec![meta]
    }
}

#[allow(deprecated)]
impl<'info, T: AccountDeserialize + Clone> ToAccountInfos<'info> for CpiAccount<'info, T> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.info.clone()]
    }
}

#[allow(deprecated)]
impl<'info, T: AccountDeserialize + Clone> AsRef<AccountInfo<'info>> for CpiAccount<'info, T> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.info
    }
}

#[allow(deprecated)]
impl<'a, T: AccountDeserialize + Clone> Deref for CpiAccount<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.account
    }
}

#[allow(deprecated)]
impl<'a, T: AccountDeserialize + Clone> DerefMut for CpiAccount<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.account
    }
}

#[allow(deprecated)]
impl<'info, T: AccountDeserialize + Clone> AccountsExit<'info> for CpiAccount<'info, T> {}

#[allow(deprecated)]
impl<'info, T> From<Account<'info, T>> for CpiAccount<'info, T>
where
    T: AccountSerialize + AccountDeserialize + Owner + Clone,
{
    fn from(a: Account<'info, T>) -> Self {
        Self::new(a.to_account_info(), Box::new(a.into_inner()))
    }
}

#[allow(deprecated)]
impl<'info, T: AccountDeserialize + Clone> Key for CpiAccount<'info, T> {
    fn key(&self) -> Pubkey {
        *self.info.key
    }
}
