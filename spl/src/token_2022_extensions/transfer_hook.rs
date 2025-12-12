use anchor_lang::prelude::UncheckedAccount;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Result, ToAccountInfo};
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
        &[
            ctx.accounts.token_program_id.to_account_info(),
            ctx.accounts.mint.to_account_info(),
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct TransferHookInitialize<'info> {
    pub token_program_id: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
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
            ctx.accounts.token_program_id.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.authority.to_account_info(),
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct TransferHookUpdate<'info> {
    pub token_program_id: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
    pub authority: UncheckedAccount<'info>,
}
