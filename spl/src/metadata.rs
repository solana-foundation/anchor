use anchor_lang::context::CpiContext;
use anchor_lang::error::ErrorCode;
use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{system_program, Accounts, Result, ToAccountInfos};
use std::ops::Deref;

pub use mpl_token_metadata;
pub use mpl_token_metadata::ID;

pub fn approve_collection_authority<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, ApproveCollectionAuthority<'info>>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::ApproveCollectionAuthority {
        collection_authority_record: *ctx.accounts.collection_authority_record.key,
        metadata: *ctx.accounts.metadata.key,
        mint: *ctx.accounts.mint.key,
        new_collection_authority: *ctx.accounts.new_collection_authority.key,
        payer: *ctx.accounts.payer.key,
        rent: None,
        system_program: system_program::ID,
        update_authority: *ctx.accounts.update_authority.key,
    }
    .instruction();
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn bubblegum_set_collection_size<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, BubblegumSetCollectionSize<'info>>,
    collection_authority_record: Option<Pubkey>,
    size: u64,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::BubblegumSetCollectionSize {
        collection_metadata: *ctx.accounts.metadata_account.key,
        collection_authority: *ctx.accounts.update_authority.key,
        collection_mint: *ctx.accounts.mint.key,
        bubblegum_signer: *ctx.accounts.bubblegum_signer.key,
        collection_authority_record,
    }
    .instruction(
        mpl_token_metadata::instructions::BubblegumSetCollectionSizeInstructionArgs {
            set_collection_size_args: mpl_token_metadata::types::SetCollectionSizeArgs { size },
        },
    );
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn burn_edition_nft<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, BurnEditionNft<'info>>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::BurnEditionNft {
        edition_marker_account: *ctx.accounts.edition_marker.key,
        master_edition_account: *ctx.accounts.master_edition.key,
        master_edition_mint: *ctx.accounts.master_edition_mint.key,
        master_edition_token_account: *ctx.accounts.master_edition_token.key,
        metadata: *ctx.accounts.metadata.key,
        owner: *ctx.accounts.owner.key,
        print_edition_account: *ctx.accounts.print_edition.key,
        print_edition_mint: *ctx.accounts.print_edition_mint.key,
        print_edition_token_account: *ctx.accounts.print_edition_token.key,
        spl_token_program: *ctx.accounts.spl_token.key,
    }
    .instruction();
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

/// Burn an NFT by closing its token, metadata and edition accounts.
///
/// The lamports of the closed accounts will be transferred to the owner.
///
/// # Note
///
/// This instruction takes an optional `collection_metadata` argument, if this argument is
/// `Some`, the `ctx` argument should also include the `collection_metadata` account in its
/// remaining accounts, otherwise the CPI will fail because [`BurnNft`] only includes required
/// accounts.
///
/// ```ignore
/// CpiContext::new(program, BurnNft { .. })
///     .with_remaining_accounts(vec![ctx.accounts.collection_metadata]);
/// ```
pub fn burn_nft<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, BurnNft<'info>>,
    collection_metadata: Option<Pubkey>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::BurnNft {
        collection_metadata,
        master_edition_account: *ctx.accounts.edition.key,
        metadata: *ctx.accounts.metadata.key,
        mint: *ctx.accounts.mint.key,
        owner: *ctx.accounts.owner.key,
        spl_token_program: *ctx.accounts.spl_token.key,
        token_account: *ctx.accounts.token.key,
    }
    .instruction();
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn create_metadata_accounts_v3<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, CreateMetadataAccountsV3<'info>>,
    data: mpl_token_metadata::types::DataV2,
    is_mutable: bool,
    update_authority_is_signer: bool,
    collection_details: Option<mpl_token_metadata::types::CollectionDetails>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::CreateMetadataAccountV3 {
        metadata: *ctx.accounts.metadata.key,
        mint: *ctx.accounts.mint.key,
        mint_authority: *ctx.accounts.mint_authority.key,
        payer: *ctx.accounts.payer.key,
        rent: None,
        system_program: system_program::ID,
        update_authority: (
            *ctx.accounts.update_authority.key,
            update_authority_is_signer,
        ),
    }
    .instruction(
        mpl_token_metadata::instructions::CreateMetadataAccountV3InstructionArgs {
            collection_details,
            data,
            is_mutable,
        },
    );
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn update_metadata_accounts_v2<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, UpdateMetadataAccountsV2<'info>>,
    new_update_authority: Option<Pubkey>,
    data: Option<mpl_token_metadata::types::DataV2>,
    primary_sale_happened: Option<bool>,
    is_mutable: Option<bool>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::UpdateMetadataAccountV2 {
        metadata: *ctx.accounts.metadata.key,
        update_authority: *ctx.accounts.update_authority.key,
    }
    .instruction(
        mpl_token_metadata::instructions::UpdateMetadataAccountV2InstructionArgs {
            new_update_authority,
            data,
            primary_sale_happened,
            is_mutable,
        },
    );
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn create_master_edition_v3<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, CreateMasterEditionV3<'info>>,
    max_supply: Option<u64>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::CreateMasterEditionV3 {
        edition: *ctx.accounts.edition.key,
        metadata: *ctx.accounts.metadata.key,
        mint: *ctx.accounts.mint.key,
        mint_authority: *ctx.accounts.mint_authority.key,
        payer: *ctx.accounts.payer.key,
        rent: None,
        system_program: system_program::ID,
        token_program: spl_token_interface::ID,
        update_authority: *ctx.accounts.update_authority.key,
    }
    .instruction(
        mpl_token_metadata::instructions::CreateMasterEditionV3InstructionArgs { max_supply },
    );
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn mint_new_edition_from_master_edition_via_token<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, MintNewEditionFromMasterEditionViaToken<'info>>,
    edition: u64,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::MintNewEditionFromMasterEditionViaToken {
        edition_mark_pda: *ctx.accounts.edition_mark_pda.key,
        master_edition: *ctx.accounts.master_edition.key,
        metadata: *ctx.accounts.metadata.key,
        new_edition: *ctx.accounts.new_edition.key,
        new_metadata: *ctx.accounts.new_metadata.key,
        new_metadata_update_authority: *ctx.accounts.new_metadata_update_authority.key,
        new_mint: *ctx.accounts.new_mint.key,
        new_mint_authority: *ctx.accounts.new_mint_authority.key,
        payer: *ctx.accounts.payer.key,
        rent: None,
        system_program: system_program::ID,
        token_account: *ctx.accounts.token_account.key,
        token_account_owner: *ctx.accounts.token_account_owner.key,
        token_program: spl_token_interface::ID,
    }
    .instruction(
        mpl_token_metadata::instructions::MintNewEditionFromMasterEditionViaTokenInstructionArgs {
            mint_new_edition_from_master_edition_via_token_args:
                mpl_token_metadata::types::MintNewEditionFromMasterEditionViaTokenArgs { edition },
        },
    );
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn revoke_collection_authority<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, RevokeCollectionAuthority<'info>>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::RevokeCollectionAuthority {
        collection_authority_record: *ctx.accounts.collection_authority_record.key,
        delegate_authority: *ctx.accounts.delegate_authority.key,
        metadata: *ctx.accounts.metadata.key,
        mint: *ctx.accounts.mint.key,
        revoke_authority: *ctx.accounts.revoke_authority.key,
    }
    .instruction();
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn set_collection_size<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, SetCollectionSize<'info>>,
    collection_authority_record: Option<Pubkey>,
    size: u64,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::SetCollectionSize {
        collection_authority: *ctx.accounts.update_authority.key,
        collection_authority_record,
        collection_metadata: *ctx.accounts.metadata.key,
        collection_mint: *ctx.accounts.mint.key,
    }
    .instruction(
        mpl_token_metadata::instructions::SetCollectionSizeInstructionArgs {
            set_collection_size_args: mpl_token_metadata::types::SetCollectionSizeArgs { size },
        },
    );
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn verify_collection<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, VerifyCollection<'info>>,
    collection_authority_record: Option<Pubkey>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::VerifyCollection {
        collection: *ctx.accounts.collection_metadata.key,
        collection_authority: *ctx.accounts.collection_authority.key,
        collection_authority_record,
        collection_master_edition_account: *ctx.accounts.collection_master_edition.key,
        collection_mint: *ctx.accounts.collection_mint.key,
        metadata: *ctx.accounts.metadata.key,
        payer: *ctx.accounts.payer.key,
    }
    .instruction();
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn verify_sized_collection_item<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, VerifySizedCollectionItem<'info>>,
    collection_authority_record: Option<Pubkey>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::VerifySizedCollectionItem {
        collection: *ctx.accounts.collection_metadata.key,
        collection_authority: *ctx.accounts.collection_authority.key,
        collection_authority_record,
        collection_master_edition_account: *ctx.accounts.collection_master_edition.key,
        collection_mint: *ctx.accounts.collection_mint.key,
        metadata: *ctx.accounts.metadata.key,
        payer: *ctx.accounts.payer.key,
    }
    .instruction();
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn set_and_verify_collection<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, SetAndVerifyCollection<'info>>,
    collection_authority_record: Option<Pubkey>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::SetAndVerifyCollection {
        collection: *ctx.accounts.collection_metadata.key,
        collection_authority: *ctx.accounts.collection_authority.key,
        collection_authority_record,
        collection_master_edition_account: *ctx.accounts.collection_master_edition.key,
        collection_mint: *ctx.accounts.collection_mint.key,
        metadata: *ctx.accounts.metadata.key,
        payer: *ctx.accounts.payer.key,
        update_authority: *ctx.accounts.update_authority.key,
    }
    .instruction();
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn set_and_verify_sized_collection_item<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, SetAndVerifySizedCollectionItem<'info>>,
    collection_authority_record: Option<Pubkey>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::SetAndVerifySizedCollectionItem {
        collection: *ctx.accounts.collection_metadata.key,
        collection_authority: *ctx.accounts.collection_authority.key,
        collection_authority_record,
        collection_master_edition_account: *ctx.accounts.collection_master_edition.key,
        collection_mint: *ctx.accounts.collection_mint.key,
        metadata: *ctx.accounts.metadata.key,
        payer: *ctx.accounts.payer.key,
        update_authority: *ctx.accounts.update_authority.key,
    }
    .instruction();
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn freeze_delegated_account<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, FreezeDelegatedAccount<'info>>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::FreezeDelegatedAccount {
        delegate: *ctx.accounts.delegate.key,
        edition: *ctx.accounts.edition.key,
        mint: *ctx.accounts.mint.key,
        token_account: *ctx.accounts.token_account.key,
        token_program: *ctx.accounts.token_program.key,
    }
    .instruction();
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn thaw_delegated_account<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, ThawDelegatedAccount<'info>>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::ThawDelegatedAccount {
        delegate: *ctx.accounts.delegate.key,
        edition: *ctx.accounts.edition.key,
        mint: *ctx.accounts.mint.key,
        token_account: *ctx.accounts.token_account.key,
        token_program: *ctx.accounts.token_program.key,
    }
    .instruction();
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn update_primary_sale_happened_via_token<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, UpdatePrimarySaleHappenedViaToken<'info>>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::UpdatePrimarySaleHappenedViaToken {
        metadata: *ctx.accounts.metadata.key,
        owner: *ctx.accounts.owner.key,
        token: *ctx.accounts.token.key,
    }
    .instruction();
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;
    Ok(())
}

pub fn set_token_standard<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, SetTokenStandard<'info>>,
    edition_account: Option<Pubkey>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::SetTokenStandard {
        edition: edition_account,
        metadata: *ctx.accounts.metadata_account.key,
        mint: *ctx.accounts.mint_account.key,
        update_authority: *ctx.accounts.update_authority.key,
    }
    .instruction();
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn sign_metadata<'info>(ctx: CpiContext<'_, '_, '_, 'info, SignMetadata<'info>>) -> Result<()> {
    let ix = mpl_token_metadata::instructions::SignMetadata {
        creator: *ctx.accounts.creator.key,
        metadata: *ctx.accounts.metadata.key,
    }
    .instruction();
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;
    Ok(())
}

pub fn remove_creator_verification<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, RemoveCreatorVerification<'info>>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::RemoveCreatorVerification {
        creator: *ctx.accounts.creator.key,
        metadata: *ctx.accounts.metadata.key,
    }
    .instruction();
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;
    Ok(())
}

pub fn utilize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, Utilize<'info>>,
    use_authority_record: Option<Pubkey>,
    burner: Option<Pubkey>,
    number_of_uses: u64,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::Utilize {
        ata_program: spl_associated_token_account_interface::program::ID,
        burner,
        metadata: *ctx.accounts.metadata.key,
        mint: *ctx.accounts.mint.key,
        owner: *ctx.accounts.owner.key,
        rent: solana_sysvar::rent::ID,
        system_program: system_program::ID,
        token_account: *ctx.accounts.token_account.key,
        token_program: spl_token_interface::ID,
        use_authority: *ctx.accounts.use_authority.key,
        use_authority_record,
    }
    .instruction(mpl_token_metadata::instructions::UtilizeInstructionArgs { number_of_uses });
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn unverify_collection<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, UnverifyCollection<'info>>,
    collection_authority_record: Option<Pubkey>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::UnverifyCollection {
        collection: *ctx.accounts.metadata.key,
        collection_authority: *ctx.accounts.collection_authority.key,
        collection_authority_record,
        collection_master_edition_account: *ctx.accounts.collection_master_edition_account.key,
        collection_mint: *ctx.accounts.collection_mint.key,
        metadata: *ctx.accounts.metadata.key,
    }
    .instruction();
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn unverify_sized_collection_item<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, UnverifySizedCollectionItem<'info>>,
    collection_authority_record: Option<Pubkey>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::UnverifySizedCollectionItem {
        collection: *ctx.accounts.metadata.key,
        collection_authority: *ctx.accounts.collection_authority.key,
        collection_authority_record,
        collection_master_edition_account: *ctx.accounts.collection_master_edition_account.key,
        collection_mint: *ctx.accounts.collection_mint.key,
        metadata: *ctx.accounts.metadata.key,
        payer: *ctx.accounts.payer.key,
    }
    .instruction();
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct ApproveCollectionAuthority<'info> {
    pub collection_authority_record: UncheckedAccount<'info>,
    pub new_collection_authority: UncheckedAccount<'info>,
    pub update_authority: UncheckedAccount<'info>,
    pub payer: UncheckedAccount<'info>,
    pub metadata: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct BubblegumSetCollectionSize<'info> {
    pub metadata_account: UncheckedAccount<'info>,
    pub update_authority: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
    pub bubblegum_signer: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct BurnEditionNft<'info> {
    pub metadata: UncheckedAccount<'info>,
    pub owner: UncheckedAccount<'info>,
    pub print_edition_mint: UncheckedAccount<'info>,
    pub master_edition_mint: UncheckedAccount<'info>,
    pub print_edition_token: UncheckedAccount<'info>,
    pub master_edition_token: UncheckedAccount<'info>,
    pub master_edition: UncheckedAccount<'info>,
    pub print_edition: UncheckedAccount<'info>,
    pub edition_marker: UncheckedAccount<'info>,
    pub spl_token: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct BurnNft<'info> {
    pub metadata: UncheckedAccount<'info>,
    pub owner: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
    pub token: UncheckedAccount<'info>,
    pub edition: UncheckedAccount<'info>,
    pub spl_token: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct CreateMetadataAccountsV3<'info> {
    pub metadata: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
    pub mint_authority: UncheckedAccount<'info>,
    pub payer: UncheckedAccount<'info>,
    pub update_authority: UncheckedAccount<'info>,
    pub system_program: UncheckedAccount<'info>,
    pub rent: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct UpdateMetadataAccountsV2<'info> {
    pub metadata: UncheckedAccount<'info>,
    pub update_authority: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct CreateMasterEditionV3<'info> {
    pub edition: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
    pub update_authority: UncheckedAccount<'info>,
    pub mint_authority: UncheckedAccount<'info>,
    pub payer: UncheckedAccount<'info>,
    pub metadata: UncheckedAccount<'info>,
    pub token_program: UncheckedAccount<'info>,
    pub system_program: UncheckedAccount<'info>,
    pub rent: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct MintNewEditionFromMasterEditionViaToken<'info> {
    pub new_metadata: UncheckedAccount<'info>,
    pub new_edition: UncheckedAccount<'info>,
    pub master_edition: UncheckedAccount<'info>,
    pub new_mint: UncheckedAccount<'info>,
    pub edition_mark_pda: UncheckedAccount<'info>,
    pub new_mint_authority: UncheckedAccount<'info>,
    pub payer: UncheckedAccount<'info>,
    pub token_account_owner: UncheckedAccount<'info>,
    pub token_account: UncheckedAccount<'info>,
    pub new_metadata_update_authority: UncheckedAccount<'info>,
    pub metadata: UncheckedAccount<'info>,
    pub token_program: UncheckedAccount<'info>,
    pub system_program: UncheckedAccount<'info>,
    pub rent: UncheckedAccount<'info>,
    //
    // Not actually used by the program but still needed because it's needed
    // for the pda calculation in the helper. :/
    //
    // The better thing to do would be to remove this and have the instruction
    // helper pass in the `edition_mark_pda` directly.
    //
    pub metadata_mint: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct RevokeCollectionAuthority<'info> {
    pub collection_authority_record: UncheckedAccount<'info>,
    pub delegate_authority: UncheckedAccount<'info>,
    pub revoke_authority: UncheckedAccount<'info>,
    pub metadata: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct SetCollectionSize<'info> {
    pub metadata: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
    pub update_authority: UncheckedAccount<'info>,
    pub system_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct SetTokenStandard<'info> {
    pub metadata_account: UncheckedAccount<'info>,
    pub update_authority: UncheckedAccount<'info>,
    pub mint_account: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct VerifyCollection<'info> {
    pub payer: UncheckedAccount<'info>,
    pub metadata: UncheckedAccount<'info>,
    pub collection_authority: UncheckedAccount<'info>,
    pub collection_mint: UncheckedAccount<'info>,
    pub collection_metadata: UncheckedAccount<'info>,
    pub collection_master_edition: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct VerifySizedCollectionItem<'info> {
    pub payer: UncheckedAccount<'info>,
    pub metadata: UncheckedAccount<'info>,
    pub collection_authority: UncheckedAccount<'info>,
    pub collection_mint: UncheckedAccount<'info>,
    pub collection_metadata: UncheckedAccount<'info>,
    pub collection_master_edition: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct SetAndVerifyCollection<'info> {
    pub metadata: UncheckedAccount<'info>,
    pub collection_authority: UncheckedAccount<'info>,
    pub payer: UncheckedAccount<'info>,
    pub update_authority: UncheckedAccount<'info>,
    pub collection_mint: UncheckedAccount<'info>,
    pub collection_metadata: UncheckedAccount<'info>,
    pub collection_master_edition: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct SetAndVerifySizedCollectionItem<'info> {
    pub metadata: UncheckedAccount<'info>,
    pub collection_authority: UncheckedAccount<'info>,
    pub payer: UncheckedAccount<'info>,
    pub update_authority: UncheckedAccount<'info>,
    pub collection_mint: UncheckedAccount<'info>,
    pub collection_metadata: UncheckedAccount<'info>,
    pub collection_master_edition: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct FreezeDelegatedAccount<'info> {
    pub metadata: UncheckedAccount<'info>,
    pub delegate: UncheckedAccount<'info>,
    pub token_account: UncheckedAccount<'info>,
    pub edition: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
    pub token_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct ThawDelegatedAccount<'info> {
    pub metadata: UncheckedAccount<'info>,
    pub delegate: UncheckedAccount<'info>,
    pub token_account: UncheckedAccount<'info>,
    pub edition: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
    pub token_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct UpdatePrimarySaleHappenedViaToken<'info> {
    pub metadata: UncheckedAccount<'info>,
    pub owner: UncheckedAccount<'info>,
    pub token: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct SignMetadata<'info> {
    pub creator: UncheckedAccount<'info>,
    pub metadata: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct RemoveCreatorVerification<'info> {
    pub creator: UncheckedAccount<'info>,
    pub metadata: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Utilize<'info> {
    pub metadata: UncheckedAccount<'info>,
    pub token_account: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
    pub use_authority: UncheckedAccount<'info>,
    pub owner: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct UnverifyCollection<'info> {
    pub metadata: UncheckedAccount<'info>,
    pub collection_authority: UncheckedAccount<'info>,
    pub collection_mint: UncheckedAccount<'info>,
    pub collection: UncheckedAccount<'info>,
    pub collection_master_edition_account: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct UnverifySizedCollectionItem<'info> {
    pub metadata: UncheckedAccount<'info>,
    pub collection_authority: UncheckedAccount<'info>,
    pub payer: UncheckedAccount<'info>,
    pub collection_mint: UncheckedAccount<'info>,
    pub collection: UncheckedAccount<'info>,
    pub collection_master_edition_account: UncheckedAccount<'info>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetadataAccount(mpl_token_metadata::accounts::Metadata);

impl anchor_lang::AccountDeserialize for MetadataAccount {
    fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        let md = Self::try_deserialize_unchecked(buf)?;
        if md.key != mpl_token_metadata::types::Key::MetadataV1 {
            return Err(ErrorCode::AccountNotInitialized.into());
        }
        Ok(md)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        let md = mpl_token_metadata::accounts::Metadata::safe_deserialize(buf)?;
        Ok(Self(md))
    }
}

impl anchor_lang::AccountSerialize for MetadataAccount {}

impl anchor_lang::Owner for MetadataAccount {
    fn owner() -> Pubkey {
        ID
    }
}

impl Deref for MetadataAccount {
    type Target = mpl_token_metadata::accounts::Metadata;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MasterEditionAccount(mpl_token_metadata::accounts::MasterEdition);

impl anchor_lang::AccountDeserialize for MasterEditionAccount {
    fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        let me = Self::try_deserialize_unchecked(buf)?;
        if me.key != mpl_token_metadata::types::Key::MasterEditionV2 {
            return Err(ErrorCode::AccountNotInitialized.into());
        }
        Ok(me)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        let result = mpl_token_metadata::accounts::MasterEdition::safe_deserialize(buf)?;
        Ok(Self(result))
    }
}

impl Deref for MasterEditionAccount {
    type Target = mpl_token_metadata::accounts::MasterEdition;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl anchor_lang::AccountSerialize for MasterEditionAccount {}

impl anchor_lang::Owner for MasterEditionAccount {
    fn owner() -> Pubkey {
        ID
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TokenRecordAccount(mpl_token_metadata::accounts::TokenRecord);

impl TokenRecordAccount {
    pub const LEN: usize = mpl_token_metadata::accounts::TokenRecord::LEN;
}
impl anchor_lang::AccountDeserialize for TokenRecordAccount {
    fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        let tr = Self::try_deserialize_unchecked(buf)?;
        if tr.key != mpl_token_metadata::types::Key::TokenRecord {
            return Err(ErrorCode::AccountNotInitialized.into());
        }
        Ok(tr)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        let tr = mpl_token_metadata::accounts::TokenRecord::safe_deserialize(buf)?;
        Ok(Self(tr))
    }
}

impl anchor_lang::AccountSerialize for TokenRecordAccount {}

impl anchor_lang::Owner for TokenRecordAccount {
    fn owner() -> Pubkey {
        ID
    }
}

impl Deref for TokenRecordAccount {
    type Target = mpl_token_metadata::accounts::TokenRecord;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct Metadata;

impl anchor_lang::Id for Metadata {
    fn id() -> Pubkey {
        ID
    }
}
