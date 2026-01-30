use crate::compat::Vec;
use crate::ToAccountMetas;
use solana_instruction::AccountMeta;

impl ToAccountMetas for AccountMeta {
    fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<AccountMeta> {
        Vec::from([self.clone()])
    }
}
