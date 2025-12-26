use anchor_lang::context::CpiContext;
use anchor_lang::prelude::AccountInfo;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{Result, ToAccountInfos, ToAccountMetas};
use spl_token_2022_interface as spl_token_2022;

pub fn group_pointer_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, GroupPointerInitialize<'info>>,
    authority: Option<Pubkey>,
    group_address: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::group_pointer::instruction::initialize(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        authority,
        group_address,
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct GroupPointerInitialize<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for GroupPointerInitialize<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.token_program_id.to_owned(), self.mint.to_owned()]
    }
}

impl<'info> ToAccountMetas for GroupPointerInitialize<'info> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_lang::prelude::AccountMeta> {
        let mut account_metas = vec![];
        account_metas.extend(self.token_program_id.to_account_metas(is_signer));
        account_metas.extend(self.mint.to_account_metas(is_signer));
        account_metas
    }
}

pub fn group_pointer_update<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, GroupPointerUpdate<'info>>,
    group_address: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::group_pointer::instruction::update(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        ctx.accounts.authority.key,
        &[ctx.accounts.authority.key],
        group_address,
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct GroupPointerUpdate<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for GroupPointerUpdate<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.token_program_id.to_owned(),
            self.mint.to_owned(),
            self.authority.to_owned(),
        ]
    }
}

impl<'info> ToAccountMetas for GroupPointerUpdate<'info> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_lang::prelude::AccountMeta> {
        let mut account_metas = vec![];
        account_metas.extend(self.token_program_id.to_account_metas(is_signer));
        account_metas.extend(self.mint.to_account_metas(is_signer));
        account_metas.extend(self.authority.to_account_metas(is_signer));
        account_metas
    }
}
