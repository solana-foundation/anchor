//! Type validating that the account is a sysvar and deserializing it

use {
    crate::{
        error::ErrorCode,
        pinocchio_runtime::{
            account_info::AccountInfo, instruction::AccountMeta, program_error::ProgramError,
            pubkey::Pubkey,
        },
        Accounts, AccountsExit, Key, Result, ToAccountInfos, ToAccountMetas,
    },
    pinocchio::sysvars::Sysvar as PinocchioSysvar,
    std::{
        collections::BTreeSet,
        fmt,
        ops::{Deref, DerefMut},
    },
};

/// Loads a sysvar value after validating the account address matches the sysvar.
pub trait SysvarFromAccount: Sized {
    fn from_account_info(info: &AccountInfo) -> Result<Self>;
}

fn program_err(e: ProgramError) -> crate::error::Error {
    e.into()
}

fn bincode_from_sysvar_account<T: solana_sysvar::SysvarSerialize>(info: &AccountInfo) -> Result<T> {
    if !T::check_id(info.address()) {
        return Err(ErrorCode::AccountSysvarMismatch.into());
    }
    let data = info.try_borrow().map_err(program_err)?;
    bincode::deserialize(&data).map_err(|_| ErrorCode::AccountDidNotDeserialize.into())
}

impl SysvarFromAccount for pinocchio::sysvars::clock::Clock {
    fn from_account_info(info: &AccountInfo) -> Result<Self> {
        use solana_sdk_ids::sysvar::clock;
        if !clock::check_id(info.address()) {
            return Err(ErrorCode::AccountSysvarMismatch.into());
        }
        PinocchioSysvar::get().map_err(program_err)
    }
}

impl SysvarFromAccount for pinocchio::sysvars::fees::Fees {
    fn from_account_info(info: &AccountInfo) -> Result<Self> {
        use solana_sdk_ids::sysvar::fees;
        if !fees::check_id(info.address()) {
            return Err(ErrorCode::AccountSysvarMismatch.into());
        }
        PinocchioSysvar::get().map_err(program_err)
    }
}

impl SysvarFromAccount for crate::Rent {
    fn from_account_info(info: &AccountInfo) -> Result<Self> {
        if !crate::rent::check_id(info.address()) {
            return Err(ErrorCode::AccountSysvarMismatch.into());
        }
        <Self as solana_sysvar::Sysvar>::get().map_err(program_err)
    }
}

impl SysvarFromAccount for solana_sysvar::epoch_schedule::EpochSchedule {
    fn from_account_info(info: &AccountInfo) -> Result<Self> {
        use solana_sdk_ids::sysvar::epoch_schedule;
        if !epoch_schedule::check_id(info.address()) {
            return Err(ErrorCode::AccountSysvarMismatch.into());
        }
        <Self as solana_sysvar::Sysvar>::get().map_err(program_err)
    }
}

impl SysvarFromAccount for solana_sysvar::rewards::Rewards {
    fn from_account_info(info: &AccountInfo) -> Result<Self> {
        bincode_from_sysvar_account(info)
    }
}

impl SysvarFromAccount for solana_sysvar::slot_history::SlotHistory {
    fn from_account_info(info: &AccountInfo) -> Result<Self> {
        use solana_sysvar::slot_history::check_id;
        if !check_id(info.address()) {
            return Err(ErrorCode::AccountSysvarMismatch.into());
        }
        Err(program_err(ProgramError::UnsupportedSysvar))
    }
}

#[allow(deprecated)]
impl SysvarFromAccount for solana_sysvar::recent_blockhashes::RecentBlockhashes {
    fn from_account_info(info: &AccountInfo) -> Result<Self> {
        bincode_from_sysvar_account(info)
    }
}

impl SysvarFromAccount for crate::stake_history::StakeHistory {
    fn from_account_info(info: &AccountInfo) -> Result<Self> {
        use crate::stake_history::check_id;
        if !check_id(info.address()) {
            return Err(ErrorCode::AccountSysvarMismatch.into());
        }
        let data = info.try_borrow().map_err(program_err)?;
        bincode::deserialize(&data)
            .map(crate::stake_history::StakeHistory)
            .map_err(|_| ErrorCode::AccountDidNotDeserialize.into())
    }
}

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
pub struct Sysvar<'info, T> {
    info: &'info AccountInfo,
    account: T,
}

impl<T: fmt::Debug> fmt::Debug for Sysvar<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sysvar")
            .field("info", &self.info)
            .field("account", &self.account)
            .finish()
    }
}

impl<T> ToAccountMetas for Sysvar<'_, T> {
    fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<AccountMeta<'_>> {
        vec![AccountMeta::readonly(self.info.address())]
    }
}

impl<'info, T> ToAccountInfos<'info> for Sysvar<'info, T> {
    fn to_account_infos(&self) -> Vec<AccountInfo> {
        vec![*self.info]
    }
}

impl<'info, T> AsRef<AccountInfo> for Sysvar<'info, T> {
    fn as_ref(&self) -> &AccountInfo {
        self.info
    }
}

impl<T> Deref for Sysvar<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.account
    }
}

impl<T> DerefMut for Sysvar<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.account
    }
}

impl<'info, T: SysvarFromAccount, B> Accounts<'info, B> for Sysvar<'info, T> {
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
        let info = &accounts[0];
        *accounts = &accounts[1..];
        let account = T::from_account_info(info)?;
        Ok(Self { info, account })
    }
}

impl<'info, T> AccountsExit<'info> for Sysvar<'info, T> {}

impl<T> Key for Sysvar<'_, T> {
    fn key(&self) -> Pubkey {
        *self.info.address()
    }
}
