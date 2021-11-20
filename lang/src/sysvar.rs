use crate::error::ErrorCode;
use crate::{Accounts, AccountsExit, Key, ToAccountInfo, ToAccountInfos, ToAccountMetas};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::fmt;
use std::ops::{Deref, DerefMut};

/// Container for sysvars.
pub struct Sysvar<'info, T: solana_program::sysvar::Sysvar> {
    info: AccountInfo<'info>,
    account: T,
}

impl<'info, T: solana_program::sysvar::Sysvar + fmt::Debug> fmt::Debug for Sysvar<'info, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sysvar")
            .field("info", &self.info)
            .field("account", &self.account)
            .finish()
    }
}

impl<'info, T: solana_program::sysvar::Sysvar> Sysvar<'info, T> {
    pub fn from_account_info(
        acc_info: &AccountInfo<'info>,
    ) -> Result<Sysvar<'info, T>, ProgramError> {
        Ok(Sysvar {
            info: acc_info.clone(),
            account: T::from_account_info(acc_info)?,
        })
    }
}

impl<'info, T: solana_program::sysvar::Sysvar> Clone for Sysvar<'info, T> {
    fn clone(&self) -> Self {
        Self {
            info: self.info.clone(),
            account: T::from_account_info(&self.info).unwrap(),
        }
    }
}

impl<'info, T: solana_program::sysvar::Sysvar> Accounts<'info> for Sysvar<'info, T> {
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
        Sysvar::from_account_info(account)
    }
}

impl<'info, T: solana_program::sysvar::Sysvar> ToAccountMetas for Sysvar<'info, T> {
    fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<AccountMeta> {
        vec![AccountMeta::new_readonly(*self.info.key, false)]
    }
}

impl<'info, T: solana_program::sysvar::Sysvar> ToAccountInfos<'info> for Sysvar<'info, T> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.info.clone()]
    }
}

impl<'info, T: solana_program::sysvar::Sysvar> AsRef<AccountInfo<'info>> for Sysvar<'info, T> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.info
    }
}

impl<'a, T: solana_program::sysvar::Sysvar> Deref for Sysvar<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.account
    }
}

impl<'a, T: solana_program::sysvar::Sysvar> DerefMut for Sysvar<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.account
    }
}

impl<'info, T: solana_program::sysvar::Sysvar> ToAccountInfo<'info> for Sysvar<'info, T> {
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.info.clone()
    }
}

impl<'info, T: solana_program::sysvar::Sysvar> AccountsExit<'info> for Sysvar<'info, T> {
    fn exit(&self, _program_id: &Pubkey) -> ProgramResult {
        // no-op
        Ok(())
    }
}

impl<'info, T: solana_program::sysvar::Sysvar> Key for Sysvar<'info, T> {
    fn key(&self) -> Pubkey {
        *self.info.key
    }
}
