use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::Result;
use anchor_lang::{context::CpiContext, Accounts};

pub fn cpi_guard_enable<'info>(ctx: CpiContext<'_, '_, '_, 'info, CpiGuard<'info>>) -> Result<()> {
    let ix = spl_token_2022::extension::cpi_guard::instruction::enable_cpi_guard(
        ctx.accounts.token_program_id.key,
        ctx.accounts.account.key,
        ctx.accounts.account.owner,
        &[],
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.account,
            ctx.accounts.owner,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn cpi_guard_disable<'info>(ctx: CpiContext<'_, '_, '_, 'info, CpiGuard<'info>>) -> Result<()> {
    let ix = spl_token_2022::extension::cpi_guard::instruction::disable_cpi_guard(
        ctx.accounts.token_program_id.key,
        ctx.accounts.account.key,
        ctx.accounts.account.owner,
        &[],
    )?;

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.account,
            ctx.accounts.owner,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct CpiGuard<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub account: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,
}
