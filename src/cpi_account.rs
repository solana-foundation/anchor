use crate::{
    AccountDeserialize, AccountSerialize, Accounts, ToAccountInfo, ToAccountInfos, ToAccountMetas,
};
use solana_program::account_info::AccountInfo;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::ops::{Deref, DerefMut};

/// Container for any account *not* owned by the current program.
#[derive(Clone)]
pub struct CpiAccount<'a, T: AccountSerialize + AccountDeserialize + Clone> {
    info: AccountInfo<'a>,
    account: T,
}

impl<'a, T: AccountSerialize + AccountDeserialize + Clone> CpiAccount<'a, T> {
    pub fn new(info: AccountInfo<'a>, account: T) -> CpiAccount<'a, T> {
        Self { info, account }
    }

    /// Deserializes the given `info` into a `CpiAccount`.
    pub fn try_from(info: &AccountInfo<'a>) -> Result<CpiAccount<'a, T>, ProgramError> {
        let mut data: &[u8] = &info.try_borrow_data()?;
        Ok(CpiAccount::new(
            info.clone(),
            T::try_deserialize(&mut data)?,
        ))
    }
}

impl<'info, T> Accounts<'info> for CpiAccount<'info, T>
where
    T: AccountSerialize + AccountDeserialize + Clone,
{
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
    ) -> Result<Self, ProgramError> {
        if accounts.len() == 0 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        let pa = CpiAccount::try_from(account)?;
        Ok(pa)
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> ToAccountMetas
    for CpiAccount<'info, T>
{
    fn to_account_metas(&self) -> Vec<AccountMeta> {
        let meta = match self.info.is_writable {
            false => AccountMeta::new_readonly(*self.info.key, self.info.is_signer),
            true => AccountMeta::new(*self.info.key, self.info.is_signer),
        };
        vec![meta]
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> ToAccountInfos<'info>
    for CpiAccount<'info, T>
{
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.info.clone()]
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> ToAccountInfo<'info>
    for CpiAccount<'info, T>
{
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.info.clone()
    }
}

impl<'a, T: AccountSerialize + AccountDeserialize + Clone> Deref for CpiAccount<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.account
    }
}

impl<'a, T: AccountSerialize + AccountDeserialize + Clone> DerefMut for CpiAccount<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.account
    }
}
