// Avoiding AccountView deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::{
    context::CpiContext,
    pinocchio_runtime::{account_view::AccountView, pubkey::Pubkey},
    Accounts, Key, Result,
};

pub fn group_pointer_initialize(
    ctx: CpiContext<'_, '_, GroupPointerInitialize>,
    authority: Option<&Pubkey>,
    group_address: Option<&Pubkey>,
) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::group_pointer::Initialize {
        token_program: ctx.accounts.token_program_id.address(),
        mint: &ctx.accounts.mint,
        authority,
        group_address,
    };
    ix.invoke().map_err(Into::into)
}

#[derive(Accounts)]
pub struct GroupPointerInitialize {
    pub token_program_id: AccountView,
    pub mint: AccountView,
}

pub fn group_pointer_update(
    ctx: CpiContext<'_, '_, GroupPointerUpdate>,
    group_address: Option<&Pubkey>,
) -> Result<()> {
    let signers: Vec<&AccountView> = ctx.remaining_accounts.iter().collect();

    let ix = pinocchio_token_2022::instructions::group_pointer::Update {
        token_program: ctx.accounts.token_program_id.address(),
        mint: &ctx.accounts.mint,
        authority: &ctx.accounts.authority,
        group_address,
        multisig_signers: &signers,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct GroupPointerUpdate {
    pub token_program_id: AccountView,
    pub mint: AccountView,
    pub authority: AccountView,
}
