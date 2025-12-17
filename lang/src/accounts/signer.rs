//! Type validating that the account signed the transaction
use crate::error::ErrorCode;
use crate::pinocchio_runtime::account_info::AccountInfo;
use crate::pinocchio_runtime::instruction::AccountMeta;
use crate::pinocchio_runtime::pubkey::Pubkey;
use crate::{Accounts, AccountsExit, Key, Result, ToAccountInfos, ToAccountMetas};
use std::collections::BTreeSet;
use std::ops::Deref;

/// Type validating that the account signed the transaction. No other ownership
/// or type checks are done. If this is used, one should not try to access the
/// underlying account data.
///
/// Checks:
///
/// - `Signer.info.is_signer == true`
///
/// # Example
/// ```ignore
/// #[account]
/// #[derive(Default)]
/// pub struct MyData {
///     pub data: u64
/// }
///
/// #[derive(Accounts)]
/// pub struct Example<'info> {
///     #[account(init, payer = payer)]
///     pub my_acc: Account<'info, MyData>,
///     #[account(mut)]
///     pub payer: Signer<'info>,
///     pub system_program: Program<'info, System>
/// }
/// ```
///
/// When creating an account with `init`, the `payer` needs to sign the transaction.
#[derive(Debug, Clone)]
pub struct Signer{
    info: AccountInfo,
}

impl Signer {
    fn new(info: AccountInfo) -> Signer {
        Self { info }
    }

    /// Deserializes the given `info` into a `Signer`.
    #[inline(never)]
    pub fn try_from(info: AccountInfo) -> Result<Signer> {
        if !info.is_signer() {
            return Err(ErrorCode::AccountNotSigner.into());
        }
        Ok(Signer::new(info))
    }
}

impl<'info, B> Accounts<'info, B> for Signer {
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
        Signer::try_from(account)
    }
}

impl<'info> AccountsExit<'info> for Signer {}

impl<'info> ToAccountMetas<'info> for Signer {
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

impl ToAccountInfos for Signer {
    fn to_account_infos(&self) -> Vec<AccountInfo> {
        vec![self.info]
    }
}

impl AsRef<AccountInfo> for Signer {
    fn as_ref(&self) -> &AccountInfo {
        &self.info
    }
}

impl Deref for Signer {
    type Target = AccountInfo;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl Key for Signer {
    fn key(&self) -> Pubkey {
        *self.info.address()
    }
}
