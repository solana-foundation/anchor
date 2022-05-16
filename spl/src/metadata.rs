use anchor_lang::context::CpiContext;
use anchor_lang::{Accounts, Result, ToAccountInfos};
use mpl_token_metadata::state::DataV2;
use mpl_token_metadata::ID;
use solana_program::account_info::AccountInfo;
use solana_program::pubkey::Pubkey;

#[derive(Clone)]
pub struct Metadata;

impl anchor_lang::Id for Metadata {
    fn id() -> Pubkey {
        ID
    }
}

pub fn create_metadata_accounts_v2<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, CreateMetadataAccountsV2<'info>>,
    data: DataV2,
    is_mutable: bool,
    update_authority_is_signer: bool,
) -> Result<()> {
    let DataV2 {
        name,
        symbol,
        uri,
        creators,
        seller_fee_basis_points,
        collection,
        uses,
    } = data;
    let ix = mpl_token_metadata::instruction::create_metadata_accounts_v2(
        ID,
        *ctx.accounts.metadata.key,
        *ctx.accounts.mint.key,
        *ctx.accounts.mint_authority.key,
        *ctx.accounts.payer.key,
        *ctx.accounts.update_authority.key,
        name,
        symbol,
        uri,
        creators,
        seller_fee_basis_points,
        update_authority_is_signer,
        is_mutable,
        collection,
        uses,
    );
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;
    Ok(())
}

pub fn update_metadata_accounts_v2<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, UpdateMetadataAccountsV2<'info>>,
    new_update_authority: Option<Pubkey>,
    data: Option<DataV2>,
    primary_sale_happened: Option<bool>,
    is_mutable: Option<bool>,
) -> Result<()> {
    let ix = mpl_token_metadata::instruction::update_metadata_accounts_v2(
        ID,
        *ctx.accounts.metadata.key,
        *ctx.accounts.update_authority.key,
        new_update_authority,
        data,
        primary_sale_happened,
        is_mutable,
    );
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;
    Ok(())
}

pub fn create_master_edition_v3<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, CreateMasterEditionV3<'info>>,
    max_supply: Option<u64>,
) -> Result<()> {
    let ix = mpl_token_metadata::instruction::create_master_edition_v3(
        ID,
        *ctx.accounts.edition.key,
        *ctx.accounts.mint.key,
        *ctx.accounts.update_authority.key,
        *ctx.accounts.mint_authority.key,
        *ctx.accounts.metadata.key,
        *ctx.accounts.payer.key,
        max_supply,
    );
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;
    Ok(())
}

pub fn mint_new_edition_from_master_edition_via_token<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, MintNewEditionFromMasterEditionViaToken<'info>>,
    edition: u64,
) -> Result<()> {
    let ix = mpl_token_metadata::instruction::mint_new_edition_from_master_edition_via_token(
        ID,
        *ctx.accounts.new_metadata.key,
        *ctx.accounts.new_edition.key,
        *ctx.accounts.master_edition.key,
        *ctx.accounts.new_mint.key,
        *ctx.accounts.new_mint_authority.key,
        *ctx.accounts.payer.key,
        *ctx.accounts.token_account_owner.key,
        *ctx.accounts.token_account.key,
        *ctx.accounts.new_metadata_update_authority.key,
        *ctx.accounts.metadata.key,
        *ctx.accounts.metadata_mint.key,
        edition,
    );

    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;
    Ok(())
}

#[derive(Accounts)]
pub struct CreateMetadataAccountsV2<'info> {
    pub metadata: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub mint_authority: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub update_authority: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct UpdateMetadataAccountsV2<'info> {
    pub metadata: AccountInfo<'info>,
    pub update_authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CreateMasterEditionV3<'info> {
    pub edition: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub update_authority: AccountInfo<'info>,
    pub mint_authority: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub metadata: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct MintNewEditionFromMasterEditionViaToken<'info> {
    pub new_metadata: AccountInfo<'info>,
    pub new_edition: AccountInfo<'info>,
    pub master_edition: AccountInfo<'info>,
    pub new_mint: AccountInfo<'info>,
    pub edition_mark_pda: AccountInfo<'info>,
    pub new_mint_authority: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub token_account_owner: AccountInfo<'info>,
    pub token_account: AccountInfo<'info>,
    pub new_metadata_update_authority: AccountInfo<'info>,
    pub metadata: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
    //
    // Not actually used by the program but still needed because it's needed
    // for the pda calculation in the helper. :/
    //
    // The better thing to do would be to remove this and have the instruction
    // helper pass in the `edition_mark_pda` directly.
    //
    pub metadata_mint: AccountInfo<'info>,
}
