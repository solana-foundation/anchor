use anchor_lang::context::CpiContext;
use anchor_lang::prelude::AccountInfo;
use anchor_lang::{Result, ToAccountInfos, ToAccountMetas};
use spl_token_2022_interface as spl_token_2022;

pub fn memo_transfer_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, MemoTransfer<'info>>,
) -> Result<()> {
    let ix = spl_token_2022::extension::memo_transfer::instruction::enable_required_transfer_memos(
        ctx.accounts.token_program_id.key,
        ctx.accounts.account.key,
        ctx.accounts.owner.key,
        &[],
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.account,
            ctx.accounts.owner,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn memo_transfer_disable<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, MemoTransfer<'info>>,
) -> Result<()> {
    let ix =
        spl_token_2022::extension::memo_transfer::instruction::disable_required_transfer_memos(
            ctx.accounts.token_program_id.key,
            ctx.accounts.account.key,
            ctx.accounts.owner.key,
            &[],
        )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.account,
            ctx.accounts.owner,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct MemoTransfer<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub account: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for MemoTransfer<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.token_program_id.to_owned(),
            self.account.to_owned(),
            self.owner.to_owned(),
        ]
    }
}

impl<'info> ToAccountMetas for MemoTransfer<'info> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_lang::prelude::AccountMeta> {
        let mut account_metas = vec![];
        account_metas.extend(self.token_program_id.to_account_metas(is_signer));
        account_metas.extend(self.account.to_account_metas(is_signer));
        account_metas.extend(self.owner.to_account_metas(is_signer));
        account_metas
    }
}
