use anchor_lang::context::CpiContext;
use anchor_lang::prelude::AccountInfo;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{Result, ToAccountInfos};
use spl_token_2022_interface as spl_token_2022;

pub fn transfer_hook_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, TransferHookInitialize<'info>>,
    authority: Option<Pubkey>,
    transfer_hook_program_id: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::transfer_hook::instruction::initialize(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        authority,
        transfer_hook_program_id,
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct TransferHookInitialize<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for TransferHookInitialize<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.token_program_id.to_owned(), self.mint.to_owned()]
    }
}

pub fn transfer_hook_update<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, TransferHookUpdate<'info>>,
    transfer_hook_program_id: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::transfer_hook::instruction::update(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        ctx.accounts.authority.key,
        &[],
        transfer_hook_program_id,
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

pub struct TransferHookUpdate<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for TransferHookUpdate<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.token_program_id.to_owned(),
            self.mint.to_owned(),
            self.authority.to_owned(),
        ]
    }
}
