use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::Result;
use anchor_lang::{context::CpiContext, Accounts};

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
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct InterestBearingMintInitialize<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
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
            ctx.accounts.token_program_id,
            ctx.accounts.mint,
            ctx.accounts.rate_authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct InterestBearingMintUpdateRate<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub rate_authority: AccountInfo<'info>,
}
