// Avoiding AccountView deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_view::AccountView;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};

pub fn permissioned_burn_initialize(
    ctx: CpiContext<'_, '_, PermissionedBurnInitialize>,
    authority: Pubkey,
) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::permissioned_burn::Initialize {
        mint: &ctx.accounts.mint,
        authority: &authority,
        token_program: ctx.accounts.token_program_id.address(),
    };
    ix.invoke().map_err(Into::into)
}

#[derive(Accounts)]
pub struct PermissionedBurnInitialize {
    pub mint: AccountView,
    pub token_program_id: AccountView,
}

pub fn permissioned_burn(ctx: CpiContext<'_, '_, PermissionedBurn>, amount: u64) -> Result<()> {
    let signers: Vec<&AccountView> = ctx.remaining_accounts.iter().collect();

    let ix = pinocchio_token_2022::instructions::permissioned_burn::Burn {
        mint: &ctx.accounts.mint,
        account: &ctx.accounts.account,
        multisig_signers: &signers,
        authority: &ctx.accounts.authority,
        permissioned_burn_authority: &ctx.accounts.permissioned_burn_authority,
        amount,
        token_program: ctx.accounts.token_program_id.address(),
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct PermissionedBurn {
    pub token_program_id: AccountView,
    pub mint: AccountView,
    pub account: AccountView,
    pub permissioned_burn_authority: AccountView,
    pub authority: AccountView,
}

pub fn permissioned_burn_checked(
    ctx: CpiContext<'_, '_, PermissionedBurnChecked>,
    amount: u64,
    decimals: u8,
) -> Result<()> {
    let signers: Vec<&AccountView> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token_2022::instructions::permissioned_burn::BurnChecked {
        mint: &ctx.accounts.mint,
        authority: &ctx.accounts.authority,
        amount,
        decimals,
        multisig_signers: &signers,
        token_program: ctx.accounts.token_program_id.address(),
        account: &ctx.accounts.account,
        permissioned_burn_authority: &ctx.accounts.permissioned_burn_authority,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct PermissionedBurnChecked {
    pub token_program_id: AccountView,
    pub mint: AccountView,
    pub account: AccountView,
    pub permissioned_burn_authority: AccountView,
    pub authority: AccountView,
}
