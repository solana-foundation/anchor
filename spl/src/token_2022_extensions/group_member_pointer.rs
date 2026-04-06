// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use {
    anchor_lang::{
        context::CpiContext,
        pinocchio_runtime::{account_view::AccountView, pubkey::Pubkey},
        Accounts, Key, Result,
    },
    spl_token_2022_interface as spl_token_2022,
};

pub fn group_member_pointer_initialize(
    ctx: CpiContext<'_, '_, GroupMemberPointerInitialize>,
    authority: Option<Pubkey>,
    member_address: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::group_member_pointer::instruction::initialize(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.mint.address(),
        authority,
        member_address,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct GroupMemberPointerInitialize {
    pub token_program_id: AccountView,
    pub mint: AccountView,
}

pub fn group_member_pointer_update(
    ctx: CpiContext<'_, '_, GroupMemberPointerUpdate>,
    member_address: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::group_member_pointer::instruction::update(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.authority.address(),
        &[],
        member_address,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.mint,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct GroupMemberPointerUpdate {
    pub token_program_id: AccountView,
    pub mint: AccountView,
    pub authority: AccountView,
}
