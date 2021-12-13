use crate::error::ErrorCode;
use crate::*;
use solana_program::account_info::AccountInfo;
use solana_program::bpf_loader_upgradeable::{self, UpgradeableLoaderState};
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::fmt;
use std::ops::Deref;

/// Account container that checks ownership on deserialization.
#[derive(Clone)]
pub struct Program<'info, T: Id + AccountDeserialize + Clone> {
    _account: T,
    info: AccountInfo<'info>,
    programdata_address: Option<Pubkey>,
}

impl<'info, T: Id + AccountDeserialize + Clone + fmt::Debug> fmt::Debug for Program<'info, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Program")
            .field("account", &self._account)
            .field("info", &self.info)
            .field("programdata_address", &self.programdata_address)
            .finish()
    }
}

impl<'a, T: Id + AccountDeserialize + Clone> Program<'a, T> {
    fn new(
        info: AccountInfo<'a>,
        _account: T,
        programdata_address: Option<Pubkey>,
    ) -> Program<'a, T> {
        Self {
            info,
            _account,
            programdata_address,
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

        // Programs have no data so use an empty slice.
        let mut empty = &[][..];
        Ok(Program::new(
            info.clone(),
            T::try_deserialize(&mut empty)?,
            programdata_address,
        ))
    }

    pub fn programdata_address(&self) -> Option<Pubkey> {
        self.programdata_address
    }
}

impl<'info, T: Id + Clone> Accounts<'info> for Program<'info, T>
where
    T: Id + AccountDeserialize,
{
    #[inline(never)]
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
        _ix_data: &[u8],
    ) -> Result<Self, ProgramError> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        Program::try_from(account)
    }
}

impl<'info, T: Id + AccountDeserialize + Clone> ToAccountMetas for Program<'info, T> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.info.is_signer);
        let meta = match self.info.is_writable {
            false => AccountMeta::new_readonly(*self.info.key, is_signer),
            true => AccountMeta::new(*self.info.key, is_signer),
        };
        vec![meta]
    }
}

impl<'info, T: Id + AccountDeserialize + Clone> ToAccountInfos<'info> for Program<'info, T> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.info.clone()]
    }
}

impl<'info, T: Id + AccountDeserialize + Clone> ToAccountInfo<'info> for Program<'info, T> {
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.info.clone()
    }
}

impl<'info, T: Id + AccountDeserialize + Clone> AsRef<AccountInfo<'info>> for Program<'info, T> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.info
    }
}

impl<'info, T: Id + AccountDeserialize + Clone> Deref for Program<'info, T> {
    type Target = AccountInfo<'info>;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl<'info, T: AccountDeserialize + Id + Clone> AccountsExit<'info> for Program<'info, T> {
    fn exit(&self, _program_id: &Pubkey) -> ProgramResult {
        // No-op.
        Ok(())
    }
}
