use crate::{Accounts, ToAccountInfo, ToAccountInfos, ToAccountMetas};
use solana_program::account_info::AccountInfo;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

impl<'info> Accounts<'info> for AccountInfo<'info> {
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
    ) -> Result<Self, ProgramError> {
        if accounts.len() == 0 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        Ok(account.clone())
    }
}

impl<'info> ToAccountMetas for AccountInfo<'info> {
    fn to_account_metas(&self) -> Vec<AccountMeta> {
        let meta = match self.is_writable {
            false => AccountMeta::new_readonly(*self.key, self.is_signer),
            true => AccountMeta::new(*self.key, self.is_signer),
        };
        vec![meta]
    }
}

impl<'info> ToAccountInfos<'info> for AccountInfo<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.clone()]
    }
}

impl<'info> ToAccountInfo<'info> for AccountInfo<'info> {
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.clone()
    }
}
