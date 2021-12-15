use crate::error::ErrorCode;
use crate::*;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::ops::Deref;

/// Type validating that the account signed the transaction. No other ownership
/// or type checks are done. If this is used, one should not try to access the
/// underlying account data.
#[derive(Debug, Clone)]
pub struct Signer<'info> {
    info: AccountInfo<'info>,
}

impl<'info> Signer<'info> {
    fn new(info: AccountInfo<'info>) -> Signer<'info> {
        Self { info }
    }

    /// Deserializes the given `info` into a `Signer`.
    #[inline(never)]
    pub fn try_from(info: &AccountInfo<'info>) -> Result<Signer<'info>, ProgramError> {
        if !info.is_signer {
            return Err(ErrorCode::AccountNotSigner.into());
        }
        Ok(Signer::new(info.clone()))
    }
}

impl<'info> Accounts<'info> for Signer<'info> {
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
        Signer::try_from(account)
    }
}

impl<'info> AccountsExit<'info> for Signer<'info> {
    fn exit(&self, _program_id: &Pubkey) -> ProgramResult {
        // No-op.
        Ok(())
    }
}

impl<'info> ToAccountMetas for Signer<'info> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.info.is_signer);
        let meta = match self.info.is_writable {
            false => AccountMeta::new_readonly(*self.info.key, is_signer),
            true => AccountMeta::new(*self.info.key, is_signer),
        };
        vec![meta]
    }
}

impl<'info> ToAccountInfos<'info> for Signer<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.info.clone()]
    }
}

impl<'info> ToAccountInfo<'info> for Signer<'info> {
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.info.clone()
    }
}

impl<'info> AsRef<AccountInfo<'info>> for Signer<'info> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.info
    }
}

impl<'info> Deref for Signer<'info> {
    type Target = AccountInfo<'info>;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl<'info> Key for Signer<'info> {
    fn key(&self) -> Pubkey {
        *self.info.key
    }
}
