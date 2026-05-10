// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use {
    anchor_lang::{
        context::CpiContext,
        solana_program::{account_info::AccountInfo, pubkey::Pubkey},
        Result, ToAccountInfos, ToAccountMetas,
    },
    spl_token_2022_interface as spl_token_2022,
};

pub fn pausable_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, PausableInitialize<'info>>,
    authority: Pubkey,
) -> Result<()> {
    let ix = spl_token_2022::extension::pausable::instruction::initialize(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        &authority,
    )?;
    anchor_lang::solana_program::program::invoke(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
    )
    .map_err(Into::into)
}

pub struct PausableInitialize<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
}

pub fn pausable_resume<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, PausableToggle<'info>>,
) -> Result<()> {
    let ix = spl_token_2022::extension::pausable::instruction::resume(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        ctx.accounts.authority.key,
        &[],
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.mint,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn pausable_pause<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, PausableToggle<'info>>,
) -> Result<()> {
    let ix = spl_token_2022::extension::pausable::instruction::pause(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        ctx.accounts.authority.key,
        &[],
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.mint,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct PausableToggle<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for PausableInitialize<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.token_program_id.to_owned(), self.mint.to_owned()]
    }
}

impl<'info> ToAccountMetas for PausableInitialize<'info> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_lang::prelude::AccountMeta> {
        let mut account_metas = vec![];
        account_metas.extend(self.token_program_id.to_account_metas(is_signer));
        account_metas.extend(self.mint.to_account_metas(is_signer));
        account_metas
    }
}

impl<'info> ToAccountInfos<'info> for PausableToggle<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.token_program_id.to_owned(),
            self.mint.to_owned(),
            self.authority.to_owned(),
        ]
    }
}

impl<'info> ToAccountMetas for PausableToggle<'info> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_lang::prelude::AccountMeta> {
        let mut account_metas = vec![];
        account_metas.extend(self.token_program_id.to_account_metas(is_signer));
        account_metas.extend(self.mint.to_account_metas(is_signer));
        account_metas.extend(self.authority.to_account_metas(is_signer));
        account_metas
    }
}
