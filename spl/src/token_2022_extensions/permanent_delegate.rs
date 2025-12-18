use anchor_lang::context::CpiContext;
use anchor_lang::prelude::AccountInfo;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{Result, ToAccountInfos};
use spl_token_2022_interface as spl_token_2022;

pub fn permanent_delegate_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, PermanentDelegateInitialize<'info>>,
    permanent_delegate: &Pubkey,
) -> Result<()> {
    let ix = spl_token_2022::instruction::initialize_permanent_delegate(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        permanent_delegate,
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct PermanentDelegateInitialize<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for PermanentDelegateInitialize<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.token_program_id.to_owned(), self.mint.to_owned()]
    }
}
