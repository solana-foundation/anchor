use anchor_lang::prelude::UncheckedAccount;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Result, ToAccountInfo};

pub fn token_group_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, TokenGroupInitialize<'info>>,
    update_authority: Option<Pubkey>,
    max_size: u64,
) -> Result<()> {
    let ix = spl_token_group_interface::instruction::initialize_group(
        ctx.accounts.program_id.key,
        ctx.accounts.group.key,
        ctx.accounts.mint.key,
        ctx.accounts.mint_authority.key,
        update_authority,
        max_size,
    );
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.program_id.to_account_info(),
            ctx.accounts.group.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct TokenGroupInitialize<'info> {
    pub program_id: UncheckedAccount<'info>,
    pub group: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
    pub mint_authority: UncheckedAccount<'info>,
}

pub fn token_member_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, TokenMemberInitialize<'info>>,
) -> Result<()> {
    let ix = spl_token_group_interface::instruction::initialize_member(
        ctx.accounts.program_id.key,
        ctx.accounts.member.key,
        ctx.accounts.member_mint.key,
        ctx.accounts.member_mint_authority.key,
        ctx.accounts.group.key,
        ctx.accounts.group_update_authority.key,
    );
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.program_id.to_account_info(),
            ctx.accounts.member.to_account_info(),
            ctx.accounts.member_mint.to_account_info(),
            ctx.accounts.member_mint_authority.to_account_info(),
            ctx.accounts.group.to_account_info(),
            ctx.accounts.group_update_authority.to_account_info(),
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct TokenMemberInitialize<'info> {
    pub program_id: UncheckedAccount<'info>,
    pub member: UncheckedAccount<'info>,
    pub member_mint: UncheckedAccount<'info>,
    pub member_mint_authority: UncheckedAccount<'info>,
    pub group: UncheckedAccount<'info>,
    pub group_update_authority: UncheckedAccount<'info>,
}
