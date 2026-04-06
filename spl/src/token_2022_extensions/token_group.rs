// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::{
    context::CpiContext,
    pinocchio_runtime::{account_view::AccountView, pubkey::Pubkey},
    Accounts, Key, Result,
};

pub fn token_group_initialize(
    ctx: CpiContext<'_, '_, TokenGroupInitialize>,
    update_authority: Option<Pubkey>,
    max_size: u64,
) -> Result<()> {
    let ix = spl_token_group_interface::instruction::initialize_group(
        *ctx.accounts.program_id.address(),
        *ctx.accounts.group.address(),
        *ctx.accounts.mint.address(),
        *ctx.accounts.mint_authority.address(),
        update_authority,
        max_size,
    );
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.program_id,
            ctx.accounts.group,
            ctx.accounts.mint,
            ctx.accounts.mint_authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct TokenGroupInitialize {
    pub program_id: AccountView,
    pub group: AccountView,
    pub mint: AccountView,
    pub mint_authority: AccountView,
}

pub fn token_member_initialize(ctx: CpiContext<'_, '_, TokenMemberInitialize>) -> Result<()> {
    let ix = spl_token_group_interface::instruction::initialize_member(
        *ctx.accounts.program_id.address(),
        *ctx.accounts.member.address(),
        *ctx.accounts.member_mint.address(),
        *ctx.accounts.member_mint_authority.address(),
        *ctx.accounts.group.address(),
        *ctx.accounts.group_update_authority.address(),
    );
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.program_id,
            ctx.accounts.member,
            ctx.accounts.member_mint,
            ctx.accounts.member_mint_authority,
            ctx.accounts.group,
            ctx.accounts.group_update_authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct TokenMemberInitialize {
    pub program_id: AccountView,
    pub member: AccountView,
    pub member_mint: AccountView,
    pub member_mint_authority: AccountView,
    pub group: AccountView,
    pub group_update_authority: AccountView,
}
