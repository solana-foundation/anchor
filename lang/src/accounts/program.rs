//! Type validating that the account is the given Program

use crate::error::ErrorCode;
use crate::*;
use solana_program::account_info::AccountInfo;
use solana_program::bpf_loader_upgradeable::{self, UpgradeableLoaderState};
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;

/// Type validating that the account is the given Program
///
/// The type has a `programdata_address` property that will be set
/// if the program is owned by the [`BPFUpgradeableLoader`](https://docs.rs/solana-program/latest/solana_program/bpf_loader_upgradeable/index.html)
/// and will contain the `programdata_address` property of the `Program` variant of the [`UpgradeableLoaderState`](https://docs.rs/solana-program/latest/solana_program/bpf_loader_upgradeable/enum.UpgradeableLoaderState.html) enum.
///
/// Checks:
///
/// - `Account.info.key == Program`
/// - `Account.info.executable == true`
///
/// # Example
/// ```ignore
///
/// #[program]
/// mod my_program {
///     fn set_admin_settings(...){...}
/// }
///
/// #[account]
/// #[derive(Default)]
/// pub struct AdminSettings {
///     ...
/// }
///
/// #[derive(Accounts)]
/// pub struct SetAdminSettings<'info> {
///     #[account(mut, seeds = [b"admin"], bump)]
///     pub admin_settings: Account<'info, AdminSettings>,
///     #[account(constraint = program.programdata_address() == Some(program_data.key()))]
///     pub program: Program<'info, MyProgram>,
///     #[account(constraint = program_data.upgrade_authority_address == Some(authority.key()))]
///     pub program_data: Account<'info, ProgramData>,
///     pub authority: Signer<'info>,
/// }
/// ```
/// The given program has a function with which the upgrade authority can set admin settings.
///
/// The required constraints are as follows:
///
/// - `program` is the account of the program itself.
/// Its constraint checks that `program_data` is the account that contains the program's upgrade authority.
/// Implicitly, this checks that `program` is a BPFUpgradeable program (`program.programdata_address()`
/// will be `None` if it's not).
/// - `program_data`'s constraint checks that its upgrade authority is the `authority` account.
/// - Finally, `authority` needs to sign the transaction.
#[derive(Clone)]
pub struct Program<'info, T: Id + Clone> {
    info: AccountInfo<'info>,
    programdata_address: Option<Pubkey>,
    _phantom: PhantomData<T>,
}

impl<'info, T: Id + Clone + fmt::Debug> fmt::Debug for Program<'info, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Program")
            .field("info", &self.info)
            .field("programdata_address", &self.programdata_address)
            .finish()
    }
}

impl<'a, T: Id + Clone> Program<'a, T> {
    fn new(info: AccountInfo<'a>, programdata_address: Option<Pubkey>) -> Program<'a, T> {
        Self {
            info,
            programdata_address,
            _phantom: PhantomData,
        }
    }

    /// Deserializes the given `info` into a `Program`.
    #[inline(never)]
    pub fn try_from(info: &AccountInfo<'a>) -> Result<Program<'a, T>, ProgramError> {
        if info.key != &T::id() {
            return Err(ErrorCode::InvalidProgramId.into());
        }
        if !info.executable {
            return Err(ErrorCode::InvalidProgramExecutable.into());
        }
        let programdata_address = if *info.owner == bpf_loader_upgradeable::ID {
            let mut data: &[u8] = &info.try_borrow_data()?;
            let upgradable_loader_state =
                UpgradeableLoaderState::try_deserialize_unchecked(&mut data)?;

            match upgradable_loader_state {
                UpgradeableLoaderState::Uninitialized
                | UpgradeableLoaderState::Buffer {
                    authority_address: _,
                }
                | UpgradeableLoaderState::ProgramData {
                    slot: _,
                    upgrade_authority_address: _,
                } => {
                    // Unreachable because check above already
                    // ensures that program is executable
                    // and therefore a program account.
                    unreachable!()
                }
                UpgradeableLoaderState::Program {
                    programdata_address,
                } => Some(programdata_address),
            }
        } else {
            None
        };

        Ok(Program::new(info.clone(), programdata_address))
    }

    pub fn programdata_address(&self) -> Option<Pubkey> {
        self.programdata_address
    }
}

impl<'info, T> Accounts<'info> for Program<'info, T>
where
    T: Id + Clone,
{
    #[inline(never)]
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
        _ix_data: &[u8],
        _bumps: &mut BTreeMap<String, u8>,
    ) -> Result<Self, ProgramError> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        Program::try_from(account)
    }
}

impl<'info, T: Id + Clone> ToAccountMetas for Program<'info, T> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.info.is_signer);
        let meta = match self.info.is_writable {
            false => AccountMeta::new_readonly(*self.info.key, is_signer),
            true => AccountMeta::new(*self.info.key, is_signer),
        };
        vec![meta]
    }
}

impl<'info, T: Id + Clone> ToAccountInfos<'info> for Program<'info, T> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.info.clone()]
    }
}

impl<'info, T: Id + Clone> AsRef<AccountInfo<'info>> for Program<'info, T> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.info
    }
}

impl<'info, T: Id + Clone> Deref for Program<'info, T> {
    type Target = AccountInfo<'info>;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl<'info, T: AccountDeserialize + Id + Clone> AccountsExit<'info> for Program<'info, T> {}
