//! Type validating that the account is a sysvar and deserializing it

use crate::pinocchio_runtime::account_info::AccountInfo;
use crate::pinocchio_runtime::instruction::AccountMeta;
use crate::pinocchio_runtime::pubkey::Pubkey;
use crate::pinocchio_runtime::sysvars::Sysvar as SolanaSysvar;
use crate::{AccountsExit, Key, ToAccountInfos, ToAccountMetas};
use std::fmt;
use std::ops::{Deref, DerefMut};

/// Type validating that the account is a sysvar and deserializing it.
///
/// If possible, sysvars should not be used via accounts
/// but by using the [`get`](https://docs.rs/solana-program/latest/solana_program/sysvar/trait.Sysvar.html#method.get)
/// function on the desired sysvar. This is because using `get`
/// does not run the risk of Anchor having a bug in its `Sysvar` type
/// and using `get` also decreases tx size, making space for other
/// accounts that cannot be requested via syscall.
///
/// # Example
/// ```ignore
/// // OK - via account in the account validation struct
/// #[derive(Accounts)]
/// pub struct Example<'info> {
///     pub clock: Sysvar<'info, Clock>
/// }
/// // BETTER - via syscall in the instruction function
/// fn better(ctx: Context<Better>) -> Result<()> {
///     let clock = Clock::get()?;
/// }
/// ```
pub struct Sysvar<'info, T: SolanaSysvar> {
    info: &'info AccountInfo,
    account: T,
}

impl<T: SolanaSysvar + fmt::Debug> fmt::Debug for Sysvar<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sysvar")
            .field("info", &self.info)
            .field("account", &self.account)
            .finish()
    }
}

impl<'info, T: SolanaSysvar> ToAccountMetas<'info> for Sysvar<'info, T> {
    fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<AccountMeta<'info>> {
        vec![AccountMeta::readonly(self.info.address())]
    }
}

impl<'info, T: SolanaSysvar> ToAccountInfos for Sysvar<'info, T> {
    fn to_account_infos(&self) -> Vec<AccountInfo> {
        vec![*self.info]
    }
}

impl<'info, T: SolanaSysvar> AsRef<AccountInfo> for Sysvar<'info, T> {
    fn as_ref(&self) -> &AccountInfo {
        self.info
    }
}

impl<T: SolanaSysvar> Deref for Sysvar<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.account
    }
}

impl<T: SolanaSysvar> DerefMut for Sysvar<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.account
    }
}

impl<'info, T: SolanaSysvar> AccountsExit<'info> for Sysvar<'info, T> {}

impl<T: SolanaSysvar> Key for Sysvar<'_, T> {
    fn key(&self) -> Pubkey {
        *self.info.address()
    }
}
