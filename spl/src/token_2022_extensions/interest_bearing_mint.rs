use anchor_lang::prelude::UncheckedAccount;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Result, ToAccountInfo};
use spl_token_2022_interface as spl_token_2022;

pub fn interest_bearing_mint_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, InterestBearingMintInitialize<'info>>,
    rate_authority: Option<Pubkey>,
    rate: i16,
) -> Result<()> {
    let ix = spl_token_2022::extension::interest_bearing_mint::instruction::initialize(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        rate_authority,
        rate,
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
pub struct InterestBearingMintInitialize<'info> {
    pub token_program_id: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
}

pub fn interest_bearing_mint_update_rate<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, InterestBearingMintUpdateRate<'info>>,
    rate: i16,
) -> Result<()> {
    let ix = spl_token_2022::extension::interest_bearing_mint::instruction::update_rate(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        ctx.accounts.rate_authority.key,
        &[],
        rate,
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.rate_authority.to_account_info(),
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct InterestBearingMintUpdateRate<'info> {
    pub token_program_id: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
    pub rate_authority: UncheckedAccount<'info>,
}
