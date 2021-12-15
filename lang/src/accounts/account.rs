use crate::error::ErrorCode;
use crate::*;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::fmt;
use std::ops::{Deref, DerefMut};

/// Account container that checks ownership on deserialization.
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
            return Err(ErrorCode::AccountNotProgramOwned.into());
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
            return Err(ErrorCode::AccountNotProgramOwned.into());
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
