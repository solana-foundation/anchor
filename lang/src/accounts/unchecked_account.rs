//! Explicit wrapper for AccountInfo types to emphasize
//! that no checks are performed

use crate::error::ErrorCode;
use crate::pinocchio_runtime::account_info::AccountInfo;
use crate::pinocchio_runtime::instruction::AccountMeta;
use crate::pinocchio_runtime::pubkey::Pubkey;
use crate::{Accounts, AccountsExit, Key, Result, ToAccountInfos, ToAccountMetas};
use std::collections::BTreeSet;
use std::ops::Deref;

/// Explicit wrapper for AccountInfo types to emphasize
/// that no checks are performed
#[derive(Debug, Clone)]
pub struct UncheckedAccount<'info>(&'info AccountInfo);

impl<'info> UncheckedAccount<'info> {
    pub fn try_from(acc_info: &'info AccountInfo) -> Self {
        Self(acc_info)
    }
}

impl<'info, B> Accounts<'info, B> for UncheckedAccount<'info> {
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &'info [AccountInfo],
        _ix_data: &[u8],
        _bumps: &mut B,
        _reallocs: &mut BTreeSet<Pubkey>,
    ) -> Result<Self> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        Ok(UncheckedAccount(account))
    }
}

impl ToAccountMetas for UncheckedAccount<'_> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.0.is_signer());
        let meta = match self.0.is_writable() {
            false => AccountMeta::new_readonly(*self.0.key(), is_signer),
            true => AccountMeta::new(*self.0.key(), is_signer),
        };
        vec![meta]
    }
}

impl<'info> ToAccountInfos<'info> for UncheckedAccount<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo> {
        vec![*self.0]
    }
}

impl<'info> AccountsExit<'info> for UncheckedAccount<'info> {}

impl<'info> AsRef<AccountInfo> for UncheckedAccount<'info> {
    fn as_ref(&self) -> &AccountInfo {
        self.0
    }
}

impl<'info> Deref for UncheckedAccount<'info> {
    type Target = AccountInfo;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl Key for UncheckedAccount<'_> {
    fn key(&self) -> Pubkey {
        *self.0.key()
    }
}
