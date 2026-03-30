//! AccountView can be used as a type but
//! [Unchecked Account](crate::accounts::unchecked_account::UncheckedAccount)
//! should be used instead.

use {
    crate::{
        error::ErrorCode,
        pinocchio_runtime::{account_view::AccountView, instruction::AccountMeta, pubkey::Pubkey},
        Accounts, AccountsExit, Key, Result, ToAccountMetas, ToAccountViews,
    },
    std::collections::BTreeSet,
};

impl<'info, B> Accounts<'info, B> for AccountView {
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
        Ok(account)
    }
}

impl ToAccountMetas for AccountView {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta<'_>> {
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

impl ToAccountViews for AccountView {
    fn to_account_views(&self) -> Vec<AccountView> {
        vec![*self]
    }
}

impl<'info> AccountsExit<'info> for AccountView {}

impl Key for AccountView {
    fn key(&self) -> Pubkey {
        *self.address()
    }
}
