use crate::{
    AccountDeserialize, AccountSerialize, Accounts, AccountsInit, CpiAccount, ToAccountInfo,
    ToAccountInfos, ToAccountMetas,
};
use solana_sdk::account_info::AccountInfo;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::program_error::ProgramError;
use solana_sdk::pubkey::Pubkey;
use std::ops::{Deref, DerefMut};

/// Container for a serializable `account`. Use this to reference any account
/// owned by the currently executing program.
#[derive(Clone)]
pub struct ProgramAccount<'a, T: AccountSerialize + AccountDeserialize + Clone> {
    info: AccountInfo<'a>,
    account: T,
}

impl<'a, T: AccountSerialize + AccountDeserialize + Clone> ProgramAccount<'a, T> {
    pub fn new(info: AccountInfo<'a>, account: T) -> ProgramAccount<'a, T> {
        Self { info, account }
    }

    /// Deserializes the given `info` into a `ProgramAccount`.
    pub fn try_from(info: &AccountInfo<'a>) -> Result<ProgramAccount<'a, T>, ProgramError> {
        let mut data: &[u8] = &info.try_borrow_data()?;
        Ok(ProgramAccount::new(
            info.clone(),
            T::try_deserialize(&mut data)?,
        ))
    }

    /// Deserializes the zero-initialized `info` into a `ProgramAccount` without
    /// checking the account type. This should only be used upon program account
    /// initialization (since the entire account data array is zeroed and thus
    /// no account type is set).
    pub fn try_from_init(info: &AccountInfo<'a>) -> Result<ProgramAccount<'a, T>, ProgramError> {
        let mut data: &[u8] = &info.try_borrow_data()?;

        // The discriminator should be zero, since we're initializing.
        let mut disc_bytes = [0u8; 8];
        disc_bytes.copy_from_slice(&data[..8]);
        let discriminator = u64::from_le_bytes(disc_bytes);
        if discriminator != 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(ProgramAccount::new(
            info.clone(),
            T::try_deserialize_unchecked(&mut data)?,
        ))
    }
}

impl<'info, T> Accounts<'info> for ProgramAccount<'info, T>
where
    T: AccountSerialize + AccountDeserialize + Clone,
{
    fn try_accounts(
        program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
    ) -> Result<Self, ProgramError> {
        if accounts.len() == 0 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        let pa = ProgramAccount::try_from(account)?;
        if pa.info.owner != program_id {}
        Ok(pa)
    }
}

impl<'info, T> AccountsInit<'info> for ProgramAccount<'info, T>
where
    T: AccountSerialize + AccountDeserialize + Clone,
{
    fn try_accounts_init(
        _program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
    ) -> Result<Self, ProgramError> {
        if accounts.len() == 0 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        ProgramAccount::try_from_init(account)
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> ToAccountMetas
    for ProgramAccount<'info, T>
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
    for ProgramAccount<'info, T>
{
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.info.clone()]
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> ToAccountInfo<'info>
    for ProgramAccount<'info, T>
{
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.info.clone()
    }
}

impl<'a, T: AccountSerialize + AccountDeserialize + Clone> Deref for ProgramAccount<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.account
    }
}

impl<'a, T: AccountSerialize + AccountDeserialize + Clone> DerefMut for ProgramAccount<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.account
    }
}

impl<'info, T> From<CpiAccount<'info, T>> for ProgramAccount<'info, T>
where
    T: AccountSerialize + AccountDeserialize + Clone,
{
    fn from(a: CpiAccount<'info, T>) -> Self {
        Self {
            info: a.to_account_info(),
            account: Deref::deref(&a).clone(),
        }
    }
}
