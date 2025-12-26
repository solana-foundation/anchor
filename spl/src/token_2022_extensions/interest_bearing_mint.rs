use anchor_lang::context::CpiContext;
use anchor_lang::prelude::AccountInfo;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{Result, ToAccountInfos, ToAccountMetas};
use spl_token_2022_interface as spl_token_2022;

pub fn interest_bearing_mint_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, InterestBearingMintInitialize<'info>>,
    rate_authority: Option<Pubkey>,
    rate: i16,
) -> Result<()> {
    let ix = spl_token_2022::extension::interest_bearing_mint::instruction::initialize(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        rate_authority,
        rate,
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct InterestBearingMintInitialize<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for InterestBearingMintInitialize<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.token_program_id.to_owned(), self.mint.to_owned()]
    }
}

impl<'info> ToAccountMetas for InterestBearingMintInitialize<'info> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_lang::prelude::AccountMeta> {
        let mut account_metas = vec![];
        account_metas.extend(self.token_program_id.to_account_metas(is_signer));
        account_metas.extend(self.mint.to_account_metas(is_signer));
        account_metas
    }
}

pub fn interest_bearing_mint_update_rate<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, InterestBearingMintUpdateRate<'info>>,
    rate: i16,
) -> Result<()> {
    let ix = spl_token_2022::extension::interest_bearing_mint::instruction::update_rate(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        ctx.accounts.rate_authority.key,
        &[],
        rate,
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.mint,
            ctx.accounts.rate_authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct InterestBearingMintUpdateRate<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub rate_authority: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for InterestBearingMintUpdateRate<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.token_program_id.to_owned(),
            self.mint.to_owned(),
            self.rate_authority.to_owned(),
        ]
    }
}

impl<'info> ToAccountMetas for InterestBearingMintUpdateRate<'info> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_lang::prelude::AccountMeta> {
        let mut account_metas = vec![];
        account_metas.extend(self.token_program_id.to_account_metas(is_signer));
        account_metas.extend(self.mint.to_account_metas(is_signer));
        account_metas.extend(self.rate_authority.to_account_metas(is_signer));
        account_metas
    }
}
