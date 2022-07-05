//! AccountInfo can be used as a type but
//! [Unchecked Account](crate::accounts::unchecked_account::UncheckedAccount)
//! should be used instead.

use crate::error::ErrorCode;
use crate::{Accounts, AccountsExit, Key, Result, ToAccountInfos, ToAccountMetas};
use solana_program::account_info::AccountInfo;
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;
use std::collections::{BTreeMap, BTreeSet};

impl<'info> Accounts<'info> for AccountInfo<'info> {
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
        _ix_data: &[u8],
        _bumps: &mut BTreeMap<String, u8>,
        _reallocs: &mut BTreeSet<Pubkey>,
    ) -> Result<Self> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        Ok(account.clone())
    }
}

impl<'info> ToAccountMetas for AccountInfo<'info> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.is_signer);
        let meta = match self.is_writable {
            false => AccountMeta::new_readonly(*self.key, is_signer),
            true => AccountMeta::new(*self.key, is_signer),
        };
        vec![meta]
    }
}

impl<'info> ToAccountInfos<'info> for AccountInfo<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.clone()]
    }
}

impl<'info> AccountsExit<'info> for AccountInfo<'info> {}

impl<'info> Key for AccountInfo<'info> {
    fn key(&self) -> Pubkey {
        *self.key
    }
}
