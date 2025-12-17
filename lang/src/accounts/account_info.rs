//! AccountInfo can be used as a type but
//! [Unchecked Account](crate::accounts::unchecked_account::UncheckedAccount)
//! should be used instead.

use crate::error::ErrorCode;
use crate::pinocchio_runtime::account_info::AccountInfo;
use crate::pinocchio_runtime::instruction::AccountMeta;
use crate::pinocchio_runtime::pubkey::Pubkey;
use crate::{Accounts, AccountsExit, Key, Result, ToAccountInfos, ToAccountMetas};
use std::collections::BTreeSet;

impl<'info, B> Accounts<'info, B> for AccountInfo {
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &[AccountInfo],
        _ix_data: &[u8],
        _bumps: &mut B,
        _reallocs: &mut BTreeSet<Pubkey>,
    ) -> Result<Self> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = accounts[0];
        *accounts = &accounts[1..];
        Ok(account)
    }
}

impl<'info> ToAccountMetas<'info> for AccountInfo {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.is_signer());
        let meta = match (self.is_writable(), is_signer) {
            (false, false) => AccountMeta::readonly(self.address()),
            (false, true) => AccountMeta::readonly_signer(self.address()),
            (true, false) => AccountMeta::writable(self.address()),
            (true, true) => AccountMeta::writable_signer(self.address()),
        };
        vec![meta]
    }
}

impl ToAccountInfos for AccountInfo {
    fn to_account_infos(&self) -> Vec<AccountInfo> {
        vec![self.clone()]
    }
}

impl<'info> AccountsExit<'info> for AccountInfo {}

impl Key for AccountInfo {
    fn key(&self) -> Pubkey {
        self.address().clone()
    }
}
