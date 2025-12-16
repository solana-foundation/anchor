// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use crate::pinocchio_runtime::account_info::AccountInfo;
use crate::pinocchio_runtime::instruction::AccountMeta;
use crate::pinocchio_runtime::pubkey::Pubkey;
use crate::{Accounts, Result, ToAccountInfos, ToAccountMetas};
use std::collections::BTreeSet;

impl<'info, T: ToAccountInfos<'info>> ToAccountInfos<'info> for Vec<T> {
    fn to_account_infos(&self) -> Vec<AccountInfo> {
        self.iter()
            .flat_map(|item| item.to_account_infos())
            .collect()
    }
}

impl<T: ToAccountMetas> ToAccountMetas for Vec<T> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        self.iter()
            .flat_map(|item| (*item).to_account_metas(is_signer))
            .collect()
    }
}

impl<'info, B, T: Accounts<'info, B>> Accounts<'info, B> for Vec<T> {
    fn try_accounts(
        program_id: &Pubkey,
        accounts: &mut &'info [AccountInfo],
        ix_data: &[u8],
        bumps: &mut B,
        reallocs: &mut BTreeSet<Pubkey>,
    ) -> Result<Self> {
        let mut vec: Vec<T> = Vec::new();
        T::try_accounts(program_id, accounts, ix_data, bumps, reallocs)
            .map(|item| vec.push(item))?;
        Ok(vec)
    }
}

#[cfg(test)]
mod tests {
    use crate::Key;

    use crate::pinocchio_runtime::pubkey::Pubkey;

    use super::*;

    #[derive(Accounts)]
    pub struct Test {
        #[account(signer)]
        test: AccountInfo,
    }

    // TODO: @Otter-0x4ka5h fix this test
    // #[test]
    // fn test_accounts_trait_for_vec() {
    //     let program_id = Pubkey::default();

    //     let key = Pubkey::default();
    //     let lamports1 = 0;
    //     let data1 = vec![0; 10];
    //     let owner = Pubkey::default();
    //     let mut raw = RuntimeAccount{borrow_state: 0, is_signer: 1, is_writable: 1, executable: 0, resize_delta: 0, address: key, owner: owner, lamports: lamports1, data_len: data1.len() as u64};
    //     let account1;
    //     unsafe {
    //         account1 = AccountInfo::new_unchecked(&mut raw);
    //     };

    //     let lamports2 = 0;
    //     let data2 = vec![0; 10];
    //     let mut raw = RuntimeAccount{borrow_state: 0, is_signer: 1, is_writable: 1, executable: 0, resize_delta: 0, address: key, owner: owner, lamports: lamports2, data_len: data2.len() as u64};
    //     let account2;
    //     unsafe {
    //         account2 = AccountInfo::new_unchecked(&mut raw);
    //     };
    //     let mut bumps = TestBumps::default();
    //     let mut reallocs = std::collections::BTreeSet::new();
    //     let mut accounts = &[account1, account2][..];
    //     let parsed_accounts =
    //         Vec::<Test>::try_accounts(&program_id, &mut accounts, &[], &mut bumps, &mut reallocs)
    //             .unwrap();

    //     assert_eq!(accounts.len(), parsed_accounts.len());
    // }

    #[test]
    #[should_panic]
    fn test_accounts_trait_for_vec_empty() {
        let program_id = Pubkey::default();
        let mut bumps = TestBumps::default();
        let mut reallocs = std::collections::BTreeSet::new();
        let mut accounts = &[][..];
        Vec::<Test>::try_accounts(&program_id, &mut accounts, &[], &mut bumps, &mut reallocs)
            .unwrap();
    }
}
