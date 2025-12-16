use crate::pinocchio_runtime::instruction::AccountMeta;
use crate::ToAccountMetas;

impl<'a> ToAccountMetas for AccountMeta<'a> {
    fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<AccountMeta> {
        vec![self.clone()]
    }
}
