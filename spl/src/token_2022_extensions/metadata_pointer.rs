// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};

pub fn metadata_pointer_initialize(
    ctx: CpiContext<'_, '_, MetadataPointer>,
    authority: Option<&Pubkey>,
    metadata_address: Option<&Pubkey>,
) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::metadata_pointer::Initialize {
        token_program: &ctx.accounts.token_program_id.address(),
        mint: &ctx.accounts.mint,
        authority: authority,
        metadata_address: metadata_address,
    };
    ix.invoke().map_err(Into::into)
}

#[derive(Accounts)]
pub struct MetadataPointer {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}

pub fn metadata_pointer_update(
    ctx: CpiContext<'_, '_, MetadataPointerUpdate>,
    metadata_address: Option<&Pubkey>,
) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token_2022::instructions::metadata_pointer::Update {
        token_program: &ctx.accounts.token_program_id.address(),
        mint: &ctx.accounts.mint,
        authority: &ctx.accounts.authority,
        metadata_address: metadata_address,
        signers: &signers,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct MetadataPointerUpdate {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
    pub authority: AccountInfo,
}
