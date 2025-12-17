//! Type validating that the account is owned by the system program

use crate::error::ErrorCode;
use crate::pinocchio_runtime::system_program;
use crate::*;
use std::ops::Deref;

/// Type validating that the account is owned by the system program
///
/// Checks:
///
/// - `SystemAccount.info.owner == SystemProgram`
#[derive(Debug, Clone)]
pub struct SystemAccount {
    info: AccountInfo,
}

impl SystemAccount {
    fn new(info: AccountInfo) -> SystemAccount {
        Self { info }
    }

    #[inline(never)]
    pub fn try_from(info: AccountInfo) -> Result<SystemAccount> {
        if info.owned_by(&system_program::ID) {
            return Err(ErrorCode::AccountNotSystemOwned.into());
        }
        Ok(SystemAccount::new(info))
    }
}

impl<'info, B> Accounts<'info, B> for SystemAccount {
    #[inline(never)]
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
        SystemAccount::try_from(account)
    }
}

impl<'info> AccountsExit<'info> for SystemAccount {}

impl<'info> ToAccountMetas<'info> for SystemAccount {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
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

impl ToAccountInfos for SystemAccount {
    fn to_account_infos(&self) -> Vec<AccountInfo> {
        vec![self.info]
    }
}

impl AsRef<AccountInfo> for SystemAccount {
    fn as_ref(&self) -> &AccountInfo {
        &self.info
    }
}

impl Deref for SystemAccount {
    type Target = AccountInfo;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl Key for SystemAccount {
    fn key(&self) -> Pubkey {
        *self.info.address()
    }
}
