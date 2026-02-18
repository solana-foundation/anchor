// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};

pub fn scaled_ui_amount_initialize(
    ctx: CpiContext<'_, '_, ScaledUiAmountInitialize>,
    authority: Option<&Pubkey>,
    multiplier: f64,
) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::scaled_ui_amount::Initialize {
        mint_account: &ctx.accounts.mint_account,
        authority: authority,
        multiplier: multiplier,
        token_program: &ctx.accounts.token_program_id.address(),
    };
    ix.invoke().map_err(Into::into)
}

#[derive(Accounts)]
pub struct ScaledUiAmountInitialize {
    pub token_program_id: AccountInfo,
    pub mint_account: AccountInfo,
}

pub fn scaled_ui_amount_update(
    ctx: CpiContext<'_, '_, ScaledUiAmountUpdate>,
    multiplier: f64,
    effective_timestamp: i64,
) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();

    let ix = pinocchio_token_2022::instructions::scaled_ui_amount::UpdateMultiplier {
        mint_account: &ctx.accounts.mint_account,
        authority: &ctx.accounts.authority,
        multiplier: multiplier,
        effective_timestamp: effective_timestamp,
        signers: &signers,
        token_program: &ctx.accounts.token_program_id.address(),
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct ScaledUiAmountUpdate {
    pub token_program_id: AccountInfo,
    pub mint_account: AccountInfo,
    pub authority: AccountInfo,
}
