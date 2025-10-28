//! Type validating that the account is a sysvar and deserializing it

use crate::error::ErrorCode;
use crate::solana_program::account_info::AccountInfo;
use crate::solana_program::instruction::AccountMeta;
use crate::solana_program::pubkey::Pubkey;
use crate::{Accounts, AccountsExit, Key, Result, ToAccountInfos, ToAccountMetas};

use solana_sysvar::{Sysvar as SolanaSysvar, SysvarSerialize as SolanaSysvarSerialize};

use std::collections::BTreeSet;
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
    info: &'info AccountInfo<'info>,
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

impl<'info, T: SolanaSysvarSerialize> Sysvar<'info, T> {
    pub fn from_account_info(acc_info: &'info AccountInfo<'info>) -> Result<Sysvar<'info, T>> {
        match T::from_account_info(acc_info) {
            Ok(val) => Ok(Sysvar {
                info: acc_info,
                account: val,
            }),
            Err(_) => Err(ErrorCode::AccountSysvarMismatch.into()),
        }
    }
}

impl<T: SolanaSysvarSerialize> Clone for Sysvar<'_, T> {
    fn clone(&self) -> Self {
        Self {
            info: self.info,
            account: T::from_account_info(self.info).unwrap(),
        }
    }
}

impl<'info, B, T: SolanaSysvarSerialize> Accounts<'info, B> for Sysvar<'info, T> {
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &'info [AccountInfo<'info>],
        _ix_data: &[u8],
        _bumps: &mut B,
        _reallocs: &mut BTreeSet<Pubkey>,
    ) -> Result<Self> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        Sysvar::from_account_info(account)
    }
}

impl<T: SolanaSysvar> ToAccountMetas for Sysvar<'_, T> {
    fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<AccountMeta> {
        vec![AccountMeta::new_readonly(*self.info.key, false)]
    }
}

impl<'info, T: SolanaSysvar> ToAccountInfos<'info> for Sysvar<'info, T> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.info.clone()]
    }
}

impl<'info, T: SolanaSysvar> AsRef<AccountInfo<'info>> for Sysvar<'info, T> {
    fn as_ref(&self) -> &AccountInfo<'info> {
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
        *self.info.key
    }
}
