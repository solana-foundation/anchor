// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};

pub fn transfer_hook_initialize(
    ctx: CpiContext<'_, '_, TransferHookInitialize>,
    authority: Option<&Pubkey>,
    transfer_hook_program_id: Option<&Pubkey>,
) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::transfer_hook::InitializeTransferHook {
        token_program: ctx.accounts.token_program_id.address(),
        mint: &ctx.accounts.mint,
        authority: authority,
        program_id: transfer_hook_program_id,
    };
    ix.invoke().map_err(Into::into)
}

#[derive(Accounts)]
pub struct TransferHookInitialize {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}

pub fn transfer_hook_update(
    ctx: CpiContext<'_, '_, TransferHookUpdate>,
    transfer_hook_program_id: Option<&Pubkey>,
) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();

    let ix = pinocchio_token_2022::instructions::transfer_hook::UpdateTransferHook {
        token_program: ctx.accounts.token_program_id.address(),
        mint: &ctx.accounts.mint,
        authority: &ctx.accounts.authority,
        transfer_hook_program: transfer_hook_program_id,
        multisig_signers: &signers,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct TransferHookUpdate {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
    pub authority: AccountInfo,
}
