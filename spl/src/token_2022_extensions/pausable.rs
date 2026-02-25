// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};

pub fn pausable_initialize(
    ctx: CpiContext<'_, '_, PausableInitialize>,
    authority: Pubkey,
) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::pausable::Initialize {
        token_program: ctx.accounts.token_program_id.address(),
        mint: &ctx.accounts.mint,
        authority: &authority,
    };
    ix.invoke().map_err(Into::into)
}

#[derive(Accounts)]
pub struct PausableInitialize {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}

pub fn pausable_pause(ctx: CpiContext<'_, '_, PausablePause>) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token_2022::instructions::pausable::Pause {
        mint: &ctx.accounts.mint,
        token_program: ctx.accounts.token_program_id.address(),
        multisig_signers: &signers,
        authority: &ctx.accounts.authority,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct PausablePause {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
    pub authority: AccountInfo,
}

pub fn pausable_resume(ctx: CpiContext<'_, '_, PausableResume>) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token_2022::instructions::pausable::Resume {
        mint: &ctx.accounts.mint,
        token_program: ctx.accounts.token_program_id.address(),
        multisig_signers: &signers,
        authority: &ctx.accounts.authority,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct PausableResume {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
    pub authority: AccountInfo,
}
