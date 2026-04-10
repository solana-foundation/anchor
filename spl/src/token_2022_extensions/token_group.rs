// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::{
    context::CpiContext,
    pinocchio_runtime::{account_info::AccountInfo, pubkey::Pubkey},
    Accounts, Result,
};

pub fn token_group_initialize(
    ctx: CpiContext<'_, '_, TokenGroupInitialize>,
    update_authority: Option<Pubkey>,
    max_size: u64,
) -> Result<()> {
    let ix = spl_token_group_interface::instruction::initialize_group(
        ctx.accounts.program_id.address(),
        ctx.accounts.group.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.mint_authority.address(),
        update_authority,
        max_size,
    );
    crate::cpi_util::invoke_signed_solana_instruction(ix,
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
    pub program_id: AccountInfo,
    pub group: AccountInfo,
    pub mint: AccountInfo,
    pub mint_authority: AccountInfo,
}

pub fn token_member_initialize(
    ctx: CpiContext<'_, '_, TokenMemberInitialize>,
) -> Result<()> {
    let ix = spl_token_group_interface::instruction::initialize_member(
        ctx.accounts.program_id.address(),
        ctx.accounts.member.address(),
        ctx.accounts.member_mint.address(),
        ctx.accounts.member_mint_authority.address(),
        ctx.accounts.group.address(),
        ctx.accounts.group_update_authority.address(),
    );
    crate::cpi_util::invoke_signed_solana_instruction(ix,
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
    pub program_id: AccountInfo,
    pub member: AccountInfo,
    pub member_mint: AccountInfo,
    pub member_mint_authority: AccountInfo,
    pub group: AccountInfo,
    pub group_update_authority: AccountInfo,
}
