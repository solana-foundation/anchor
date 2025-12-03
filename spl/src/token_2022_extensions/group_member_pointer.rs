use anchor_lang::prelude::UncheckedAccount;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Result, ToAccountInfo};
use spl_token_2022_interface as spl_token_2022;

pub fn group_member_pointer_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, GroupMemberPointerInitialize<'info>>,
    authority: Option<Pubkey>,
    member_address: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::group_member_pointer::instruction::initialize(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        authority,
        member_address,
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id.to_account_info(),
            ctx.accounts.mint.to_account_info(),
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct GroupMemberPointerInitialize<'info> {
    pub token_program_id: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
}

pub fn group_member_pointer_update<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, GroupMemberPointerUpdate<'info>>,
    member_address: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::group_member_pointer::instruction::update(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        ctx.accounts.authority.key,
        &[],
        member_address,
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.authority.to_account_info(),
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct GroupMemberPointerUpdate<'info> {
    pub token_program_id: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
    pub authority: UncheckedAccount<'info>,
}
