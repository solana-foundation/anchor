use crate::pinocchio_runtime::instruction::AccountMeta;
use crate::ToAccountMetas;

impl<'info> ToAccountMetas<'info> for AccountMeta<'info> {
    fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<AccountMeta<'info>> {
        vec![self.clone()]
    }
}
