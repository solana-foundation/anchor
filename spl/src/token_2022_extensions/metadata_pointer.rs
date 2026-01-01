use anchor_lang::context::CpiContext;
use anchor_lang::prelude::AccountInfo;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{Result, ToAccountInfos, ToAccountMetas};
use spl_token_2022_interface as spl_token_2022;

pub fn metadata_pointer_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, MetadataPointerInitialize<'info>>,
    authority: Option<Pubkey>,
    metadata_address: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::metadata_pointer::instruction::initialize(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        authority,
        metadata_address,
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct MetadataPointerInitialize<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for MetadataPointerInitialize<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.token_program_id.to_owned(), self.mint.to_owned()]
    }
}

impl<'info> ToAccountMetas for MetadataPointerInitialize<'info> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_lang::prelude::AccountMeta> {
        let mut account_metas = vec![];
        account_metas.extend(self.token_program_id.to_account_metas(is_signer));
        account_metas.extend(self.mint.to_account_metas(is_signer));
        account_metas
    }
}
