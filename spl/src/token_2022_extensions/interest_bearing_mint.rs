// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use {
    anchor_lang::{
        context::CpiContext,
        pinocchio_runtime::{account_info::AccountInfo, pubkey::Pubkey},
        Accounts, Result,
    },
    spl_token_2022_interface as spl_token_2022,
};

pub fn interest_bearing_mint_initialize(
    ctx: CpiContext<'_, '_, InterestBearingMintInitialize>,
    rate_authority: Option<Pubkey>,
    rate: i16,
) -> Result<()> {
    let ix = spl_token_2022::extension::interest_bearing_mint::instruction::initialize(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.mint.address(),
        rate_authority,
        rate,
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct InterestBearingMintInitialize {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}

pub fn interest_bearing_mint_update_rate(
    ctx: CpiContext<'_, '_, InterestBearingMintUpdateRate>,
    rate: i16,
) -> Result<()> {
    let ix = spl_token_2022::extension::interest_bearing_mint::instruction::update_rate(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.rate_authority.address(),
        &[],
        rate,
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(ix,
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
pub struct InterestBearingMintUpdateRate {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
    pub rate_authority: AccountInfo,
}
