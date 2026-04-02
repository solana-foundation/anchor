//! Explicit wrapper for AccountView types to emphasize
//! that no checks are performed.

use {
    crate::{
        error::ErrorCode,
        pinocchio_runtime::{account_view::AccountView, instruction::AccountMeta, pubkey::Pubkey},
        Accounts, AccountsExit, Key, Result, ToAccountMetas, ToAccountViews,
    },
    std::{collections::BTreeSet, ops::Deref},
};

/// Explicit wrapper for AccountView types to emphasize
/// that no checks are performed.
#[derive(Debug, Clone)]
pub struct UncheckedAccount<'info>(AccountView, std::marker::PhantomData<&'info ()>);

impl<'info> UncheckedAccount<'info> {
    pub fn try_from(acc_info: AccountView) -> Self {
        Self(acc_info, std::marker::PhantomData)
    }
}

impl<'info, B> Accounts<'info, B> for UncheckedAccount<'info> {
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
        Ok(UncheckedAccount::try_from(account))
    }
}

impl ToAccountMetas for UncheckedAccount<'_> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta<'_>> {
        let is_signer = is_signer.unwrap_or(self.0.is_signer());
        let meta = match (self.0.is_writable(), is_signer) {
            (false, false) => AccountMeta::readonly(self.0.address()),
            (false, true) => AccountMeta::readonly_signer(self.0.address()),
            (true, false) => AccountMeta::writable(self.0.address()),
            (true, true) => AccountMeta::writable_signer(self.0.address()),
        };
        vec![meta]
    }
}

impl ToAccountViews for UncheckedAccount<'_> {
    fn to_account_views(&self) -> Vec<AccountView> {
        vec![self.0]
    }
}

impl<'info> AccountsExit<'info> for UncheckedAccount<'info> {}

impl AsRef<AccountView> for UncheckedAccount<'_> {
    fn as_ref(&self) -> &AccountView {
        &self.0
    }
}

impl Deref for UncheckedAccount<'_> {
    type Target = AccountView;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Key for UncheckedAccount<'_> {
    fn key(&self) -> Pubkey {
        *self.0.address()
    }
}
