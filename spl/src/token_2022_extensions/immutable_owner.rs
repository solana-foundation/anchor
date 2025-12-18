use anchor_lang::context::CpiContext;
use anchor_lang::prelude::AccountInfo;
use anchor_lang::{Result, ToAccountInfos};
use spl_token_2022_interface as spl_token_2022;

pub fn immutable_owner_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, ImmutableOwnerInitialize<'info>>,
) -> Result<()> {
    let ix = spl_token_2022::instruction::initialize_immutable_owner(
        ctx.accounts.token_program_id.key,
        ctx.accounts.token_account.key,
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.token_account],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct ImmutableOwnerInitialize<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub token_account: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for ImmutableOwnerInitialize<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.token_program_id.to_owned(),
            self.token_account.to_owned(),
        ]
    }
}
