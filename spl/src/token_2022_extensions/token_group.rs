use anchor_lang::context::CpiContext;
use anchor_lang::prelude::AccountInfo;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{Result, ToAccountInfos, ToAccountMetas};

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
            ctx.accounts.program_id,
            ctx.accounts.group,
            ctx.accounts.mint,
            ctx.accounts.mint_authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct TokenGroupInitialize<'info> {
    pub program_id: AccountInfo<'info>,
    pub group: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub mint_authority: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for TokenGroupInitialize<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.program_id.to_owned(),
            self.group.to_owned(),
            self.mint.to_owned(),
            self.mint_authority.to_owned(),
        ]
    }
}

impl<'info> ToAccountMetas for TokenGroupInitialize<'info> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_lang::prelude::AccountMeta> {
        let mut account_metas = vec![];
        account_metas.extend(self.program_id.to_account_metas(is_signer));
        account_metas.extend(self.group.to_account_metas(is_signer));
        account_metas.extend(self.mint.to_account_metas(is_signer));
        account_metas.extend(self.mint_authority.to_account_metas(is_signer));
        account_metas
    }
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

pub struct TokenMemberInitialize<'info> {
    pub program_id: AccountInfo<'info>,
    pub member: AccountInfo<'info>,
    pub member_mint: AccountInfo<'info>,
    pub member_mint_authority: AccountInfo<'info>,
    pub group: AccountInfo<'info>,
    pub group_update_authority: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for TokenMemberInitialize<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.program_id.to_owned(),
            self.member.to_owned(),
            self.member_mint.to_owned(),
            self.member_mint_authority.to_owned(),
            self.group.to_owned(),
            self.group_update_authority.to_owned(),
        ]
    }
}

impl<'info> ToAccountMetas for TokenMemberInitialize<'info> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_lang::prelude::AccountMeta> {
        let mut account_metas = vec![];
        account_metas.extend(self.program_id.to_account_metas(is_signer));
        account_metas.extend(self.member.to_account_metas(is_signer));
        account_metas.extend(self.member_mint.to_account_metas(is_signer));
        account_metas.extend(self.member_mint_authority.to_account_metas(is_signer));
        account_metas.extend(self.group.to_account_metas(is_signer));
        account_metas.extend(self.group_update_authority.to_account_metas(is_signer));
        account_metas
    }
}
