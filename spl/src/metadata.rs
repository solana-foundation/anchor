pub use mpl_token_metadata::{self, ID};
use {
    anchor_lang::{
        context::CpiContext,
        error::ErrorCode,
        pinocchio_runtime::{account_info::AccountInfo, pubkey::Pubkey},
        system_program, Accounts, Result, ToAccountInfos,
    },
    std::ops::Deref,
};

pub fn approve_collection_authority(
    ctx: CpiContext<'_, '_, ApproveCollectionAuthority>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::ApproveCollectionAuthority {
        collection_authority_record: *ctx.accounts.collection_authority_record.address(),
        metadata: *ctx.accounts.metadata.address(),
        mint: *ctx.accounts.mint.address(),
        new_collection_authority: *ctx.accounts.new_collection_authority.address(),
        payer: *ctx.accounts.payer.address(),
        rent: None,
        system_program: system_program::ID,
        update_authority: *ctx.accounts.update_authority.address(),
    }
    .instruction();
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn bubblegum_set_collection_size(
    ctx: CpiContext<'_, '_, BubblegumSetCollectionSize>,
    collection_authority_record: Option<Pubkey>,
    size: u64,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::BubblegumSetCollectionSize {
        collection_metadata: *ctx.accounts.metadata_account.address(),
        collection_authority: *ctx.accounts.update_authority.address(),
        collection_mint: *ctx.accounts.mint.address(),
        bubblegum_signer: *ctx.accounts.bubblegum_signer.address(),
        collection_authority_record,
    }
    .instruction(
        mpl_token_metadata::instructions::BubblegumSetCollectionSizeInstructionArgs {
            set_collection_size_args: mpl_token_metadata::types::SetCollectionSizeArgs { size },
        },
    );
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn burn_edition_nft(ctx: CpiContext<'_, '_, BurnEditionNft>) -> Result<()> {
    let ix = mpl_token_metadata::instructions::BurnEditionNft {
        edition_marker_account: *ctx.accounts.edition_marker.address(),
        master_edition_account: *ctx.accounts.master_edition.address(),
        master_edition_mint: *ctx.accounts.master_edition_mint.address(),
        master_edition_token_account: *ctx.accounts.master_edition_token.address(),
        metadata: *ctx.accounts.metadata.address(),
        owner: *ctx.accounts.owner.address(),
        print_edition_account: *ctx.accounts.print_edition.address(),
        print_edition_mint: *ctx.accounts.print_edition_mint.address(),
        print_edition_token_account: *ctx.accounts.print_edition_token.address(),
        spl_token_program: *ctx.accounts.spl_token.address(),
    }
    .instruction();
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
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
pub fn burn_nft(
    ctx: CpiContext<'_, '_, BurnNft>,
    collection_metadata: Option<Pubkey>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::BurnNft {
        collection_metadata,
        master_edition_account: *ctx.accounts.edition.address(),
        metadata: *ctx.accounts.metadata.address(),
        mint: *ctx.accounts.mint.address(),
        owner: *ctx.accounts.owner.address(),
        spl_token_program: *ctx.accounts.spl_token.address(),
        token_account: *ctx.accounts.token.address(),
    }
    .instruction();
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn create_metadata_accounts_v3(
    ctx: CpiContext<'_, '_, CreateMetadataAccountsV3>,
    data: mpl_token_metadata::types::DataV2,
    is_mutable: bool,
    update_authority_is_signer: bool,
    collection_details: Option<mpl_token_metadata::types::CollectionDetails>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::CreateMetadataAccountV3 {
        metadata: *ctx.accounts.metadata.address(),
        mint: *ctx.accounts.mint.address(),
        mint_authority: *ctx.accounts.mint_authority.address(),
        payer: *ctx.accounts.payer.address(),
        rent: None,
        system_program: system_program::ID,
        update_authority: (
            *ctx.accounts.update_authority.address(),
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
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn update_metadata_accounts_v2(
    ctx: CpiContext<'_, '_, UpdateMetadataAccountsV2>,
    new_update_authority: Option<Pubkey>,
    data: Option<mpl_token_metadata::types::DataV2>,
    primary_sale_happened: Option<bool>,
    is_mutable: Option<bool>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::UpdateMetadataAccountV2 {
        metadata: *ctx.accounts.metadata.address(),
        update_authority: *ctx.accounts.update_authority.address(),
    }
    .instruction(
        mpl_token_metadata::instructions::UpdateMetadataAccountV2InstructionArgs {
            new_update_authority,
            data,
            primary_sale_happened,
            is_mutable,
        },
    );
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn create_master_edition_v3(
    ctx: CpiContext<'_, '_, CreateMasterEditionV3>,
    max_supply: Option<u64>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::CreateMasterEditionV3 {
        edition: *ctx.accounts.edition.address(),
        metadata: *ctx.accounts.metadata.address(),
        mint: *ctx.accounts.mint.address(),
        mint_authority: *ctx.accounts.mint_authority.address(),
        payer: *ctx.accounts.payer.address(),
        rent: None,
        system_program: system_program::ID,
        token_program: spl_token_interface::ID,
        update_authority: *ctx.accounts.update_authority.address(),
    }
    .instruction(
        mpl_token_metadata::instructions::CreateMasterEditionV3InstructionArgs { max_supply },
    );
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn mint_new_edition_from_master_edition_via_token(
    ctx: CpiContext<'_, '_, MintNewEditionFromMasterEditionViaToken>,
    edition: u64,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::MintNewEditionFromMasterEditionViaToken {
        edition_mark_pda: *ctx.accounts.edition_mark_pda.address(),
        master_edition: *ctx.accounts.master_edition.address(),
        metadata: *ctx.accounts.metadata.address(),
        new_edition: *ctx.accounts.new_edition.address(),
        new_metadata: *ctx.accounts.new_metadata.address(),
        new_metadata_update_authority: *ctx.accounts.new_metadata_update_authority.address(),
        new_mint: *ctx.accounts.new_mint.address(),
        new_mint_authority: *ctx.accounts.new_mint_authority.address(),
        payer: *ctx.accounts.payer.address(),
        rent: None,
        system_program: system_program::ID,
        token_account: *ctx.accounts.token_account.address(),
        token_account_owner: *ctx.accounts.token_account_owner.address(),
        token_program: spl_token_interface::ID,
    }
    .instruction(
        mpl_token_metadata::instructions::MintNewEditionFromMasterEditionViaTokenInstructionArgs {
            mint_new_edition_from_master_edition_via_token_args:
                mpl_token_metadata::types::MintNewEditionFromMasterEditionViaTokenArgs { edition },
        },
    );
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn revoke_collection_authority(
    ctx: CpiContext<'_, '_, RevokeCollectionAuthority>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::RevokeCollectionAuthority {
        collection_authority_record: *ctx.accounts.collection_authority_record.address(),
        delegate_authority: *ctx.accounts.delegate_authority.address(),
        metadata: *ctx.accounts.metadata.address(),
        mint: *ctx.accounts.mint.address(),
        revoke_authority: *ctx.accounts.revoke_authority.address(),
    }
    .instruction();
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn set_collection_size(
    ctx: CpiContext<'_, '_, SetCollectionSize>,
    collection_authority_record: Option<Pubkey>,
    size: u64,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::SetCollectionSize {
        collection_authority: *ctx.accounts.update_authority.address(),
        collection_authority_record,
        collection_metadata: *ctx.accounts.metadata.address(),
        collection_mint: *ctx.accounts.mint.address(),
    }
    .instruction(
        mpl_token_metadata::instructions::SetCollectionSizeInstructionArgs {
            set_collection_size_args: mpl_token_metadata::types::SetCollectionSizeArgs { size },
        },
    );
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn verify_collection(
    ctx: CpiContext<'_, '_, VerifyCollection>,
    collection_authority_record: Option<Pubkey>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::VerifyCollection {
        collection: *ctx.accounts.collection_metadata.address(),
        collection_authority: *ctx.accounts.collection_authority.address(),
        collection_authority_record,
        collection_master_edition_account: *ctx.accounts.collection_master_edition.address(),
        collection_mint: *ctx.accounts.collection_mint.address(),
        metadata: *ctx.accounts.metadata.address(),
        payer: *ctx.accounts.payer.address(),
    }
    .instruction();
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn verify_sized_collection_item(
    ctx: CpiContext<'_, '_, VerifySizedCollectionItem>,
    collection_authority_record: Option<Pubkey>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::VerifySizedCollectionItem {
        collection: *ctx.accounts.collection_metadata.address(),
        collection_authority: *ctx.accounts.collection_authority.address(),
        collection_authority_record,
        collection_master_edition_account: *ctx.accounts.collection_master_edition.address(),
        collection_mint: *ctx.accounts.collection_mint.address(),
        metadata: *ctx.accounts.metadata.address(),
        payer: *ctx.accounts.payer.address(),
    }
    .instruction();
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn set_and_verify_collection(
    ctx: CpiContext<'_, '_, SetAndVerifyCollection>,
    collection_authority_record: Option<Pubkey>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::SetAndVerifyCollection {
        collection: *ctx.accounts.collection_metadata.address(),
        collection_authority: *ctx.accounts.collection_authority.address(),
        collection_authority_record,
        collection_master_edition_account: *ctx.accounts.collection_master_edition.address(),
        collection_mint: *ctx.accounts.collection_mint.address(),
        metadata: *ctx.accounts.metadata.address(),
        payer: *ctx.accounts.payer.address(),
        update_authority: *ctx.accounts.update_authority.address(),
    }
    .instruction();
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn set_and_verify_sized_collection_item(
    ctx: CpiContext<'_, '_, SetAndVerifySizedCollectionItem>,
    collection_authority_record: Option<Pubkey>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::SetAndVerifySizedCollectionItem {
        collection: *ctx.accounts.collection_metadata.address(),
        collection_authority: *ctx.accounts.collection_authority.address(),
        collection_authority_record,
        collection_master_edition_account: *ctx.accounts.collection_master_edition.address(),
        collection_mint: *ctx.accounts.collection_mint.address(),
        metadata: *ctx.accounts.metadata.address(),
        payer: *ctx.accounts.payer.address(),
        update_authority: *ctx.accounts.update_authority.address(),
    }
    .instruction();
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn freeze_delegated_account(
    ctx: CpiContext<'_, '_, FreezeDelegatedAccount>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::FreezeDelegatedAccount {
        delegate: *ctx.accounts.delegate.address(),
        edition: *ctx.accounts.edition.address(),
        mint: *ctx.accounts.mint.address(),
        token_account: *ctx.accounts.token_account.address(),
        token_program: *ctx.accounts.token_program.address(),
    }
    .instruction();
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn thaw_delegated_account(
    ctx: CpiContext<'_, '_, ThawDelegatedAccount>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::ThawDelegatedAccount {
        delegate: *ctx.accounts.delegate.address(),
        edition: *ctx.accounts.edition.address(),
        mint: *ctx.accounts.mint.address(),
        token_account: *ctx.accounts.token_account.address(),
        token_program: *ctx.accounts.token_program.address(),
    }
    .instruction();
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn update_primary_sale_happened_via_token(
    ctx: CpiContext<'_, '_, UpdatePrimarySaleHappenedViaToken>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::UpdatePrimarySaleHappenedViaToken {
        metadata: *ctx.accounts.metadata.address(),
        owner: *ctx.accounts.owner.address(),
        token: *ctx.accounts.token.address(),
    }
    .instruction();
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn set_token_standard(
    ctx: CpiContext<'_, '_, SetTokenStandard>,
    edition_account: Option<Pubkey>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::SetTokenStandard {
        edition: edition_account,
        metadata: *ctx.accounts.metadata_account.address(),
        mint: *ctx.accounts.mint_account.address(),
        update_authority: *ctx.accounts.update_authority.address(),
    }
    .instruction();
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn sign_metadata(ctx: CpiContext<'_, '_, SignMetadata>) -> Result<()> {
    let ix = mpl_token_metadata::instructions::SignMetadata {
        creator: *ctx.accounts.creator.address(),
        metadata: *ctx.accounts.metadata.address(),
    }
    .instruction();
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn remove_creator_verification(
    ctx: CpiContext<'_, '_, RemoveCreatorVerification>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::RemoveCreatorVerification {
        creator: *ctx.accounts.creator.address(),
        metadata: *ctx.accounts.metadata.address(),
    }
    .instruction();
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn utilize(
    ctx: CpiContext<'_, '_, Utilize>,
    use_authority_record: Option<Pubkey>,
    burner: Option<Pubkey>,
    number_of_uses: u64,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::Utilize {
        ata_program: spl_associated_token_account_interface::program::ID,
        burner,
        metadata: *ctx.accounts.metadata.address(),
        mint: *ctx.accounts.mint.address(),
        owner: *ctx.accounts.owner.address(),
        rent: solana_sysvar::rent::ID,
        system_program: system_program::ID,
        token_account: *ctx.accounts.token_account.address(),
        token_program: spl_token_interface::ID,
        use_authority: *ctx.accounts.use_authority.address(),
        use_authority_record,
    }
    .instruction(mpl_token_metadata::instructions::UtilizeInstructionArgs { number_of_uses });
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn unverify_collection(
    ctx: CpiContext<'_, '_, UnverifyCollection>,
    collection_authority_record: Option<Pubkey>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::UnverifyCollection {
        collection: *ctx.accounts.metadata.address(),
        collection_authority: *ctx.accounts.collection_authority.address(),
        collection_authority_record,
        collection_master_edition_account: *ctx
            .accounts
            .collection_master_edition_account
            .address(),
        collection_mint: *ctx.accounts.collection_mint.address(),
        metadata: *ctx.accounts.metadata.address(),
    }
    .instruction();
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn unverify_sized_collection_item(
    ctx: CpiContext<'_, '_, UnverifySizedCollectionItem>,
    collection_authority_record: Option<Pubkey>,
) -> Result<()> {
    let ix = mpl_token_metadata::instructions::UnverifySizedCollectionItem {
        collection: *ctx.accounts.metadata.address(),
        collection_authority: *ctx.accounts.collection_authority.address(),
        collection_authority_record,
        collection_master_edition_account: *ctx
            .accounts
            .collection_master_edition_account
            .address(),
        collection_mint: *ctx.accounts.collection_mint.address(),
        metadata: *ctx.accounts.metadata.address(),
        payer: *ctx.accounts.payer.address(),
    }
    .instruction();
    let cpi_accounts = anchor_lang::ToAccountInfos::to_account_infos(&ctx);
    crate::cpi_util::invoke_signed_solana_instruction(ix, cpi_accounts.as_slice(), ctx.signer_seeds)
        .map_err(Into::into)
}

#[derive(Accounts)]
pub struct ApproveCollectionAuthority {
    pub collection_authority_record: AccountInfo,
    pub new_collection_authority: AccountInfo,
    pub update_authority: AccountInfo,
    pub payer: AccountInfo,
    pub metadata: AccountInfo,
    pub mint: AccountInfo,
}

#[derive(Accounts)]
pub struct BubblegumSetCollectionSize {
    pub metadata_account: AccountInfo,
    pub update_authority: AccountInfo,
    pub mint: AccountInfo,
    pub bubblegum_signer: AccountInfo,
}

#[derive(Accounts)]
pub struct BurnEditionNft {
    pub metadata: AccountInfo,
    pub owner: AccountInfo,
    pub print_edition_mint: AccountInfo,
    pub master_edition_mint: AccountInfo,
    pub print_edition_token: AccountInfo,
    pub master_edition_token: AccountInfo,
    pub master_edition: AccountInfo,
    pub print_edition: AccountInfo,
    pub edition_marker: AccountInfo,
    pub spl_token: AccountInfo,
}

#[derive(Accounts)]
pub struct BurnNft {
    pub metadata: AccountInfo,
    pub owner: AccountInfo,
    pub mint: AccountInfo,
    pub token: AccountInfo,
    pub edition: AccountInfo,
    pub spl_token: AccountInfo,
}

#[derive(Accounts)]
pub struct CreateMetadataAccountsV3 {
    pub metadata: AccountInfo,
    pub mint: AccountInfo,
    pub mint_authority: AccountInfo,
    pub payer: AccountInfo,
    pub update_authority: AccountInfo,
    pub system_program: AccountInfo,
    pub rent: AccountInfo,
}

#[derive(Accounts)]
pub struct UpdateMetadataAccountsV2 {
    pub metadata: AccountInfo,
    pub update_authority: AccountInfo,
}

#[derive(Accounts)]
pub struct CreateMasterEditionV3 {
    pub edition: AccountInfo,
    pub mint: AccountInfo,
    pub update_authority: AccountInfo,
    pub mint_authority: AccountInfo,
    pub payer: AccountInfo,
    pub metadata: AccountInfo,
    pub token_program: AccountInfo,
    pub system_program: AccountInfo,
    pub rent: AccountInfo,
}

#[derive(Accounts)]
pub struct MintNewEditionFromMasterEditionViaToken {
    pub new_metadata: AccountInfo,
    pub new_edition: AccountInfo,
    pub master_edition: AccountInfo,
    pub new_mint: AccountInfo,
    pub edition_mark_pda: AccountInfo,
    pub new_mint_authority: AccountInfo,
    pub payer: AccountInfo,
    pub token_account_owner: AccountInfo,
    pub token_account: AccountInfo,
    pub new_metadata_update_authority: AccountInfo,
    pub metadata: AccountInfo,
    pub token_program: AccountInfo,
    pub system_program: AccountInfo,
    pub rent: AccountInfo,
    //
    // Not actually used by the program but still needed because it's needed
    // for the pda calculation in the helper. :/
    //
    // The better thing to do would be to remove this and have the instruction
    // helper pass in the `edition_mark_pda` directly.
    //
    pub metadata_mint: AccountInfo,
}

#[derive(Accounts)]
pub struct RevokeCollectionAuthority {
    pub collection_authority_record: AccountInfo,
    pub delegate_authority: AccountInfo,
    pub revoke_authority: AccountInfo,
    pub metadata: AccountInfo,
    pub mint: AccountInfo,
}

#[derive(Accounts)]
pub struct SetCollectionSize {
    pub metadata: AccountInfo,
    pub mint: AccountInfo,
    pub update_authority: AccountInfo,
    pub system_program: AccountInfo,
}

#[derive(Accounts)]
pub struct SetTokenStandard {
    pub metadata_account: AccountInfo,
    pub update_authority: AccountInfo,
    pub mint_account: AccountInfo,
}

#[derive(Accounts)]
pub struct VerifyCollection {
    pub payer: AccountInfo,
    pub metadata: AccountInfo,
    pub collection_authority: AccountInfo,
    pub collection_mint: AccountInfo,
    pub collection_metadata: AccountInfo,
    pub collection_master_edition: AccountInfo,
}

#[derive(Accounts)]
pub struct VerifySizedCollectionItem {
    pub payer: AccountInfo,
    pub metadata: AccountInfo,
    pub collection_authority: AccountInfo,
    pub collection_mint: AccountInfo,
    pub collection_metadata: AccountInfo,
    pub collection_master_edition: AccountInfo,
}

#[derive(Accounts)]
pub struct SetAndVerifyCollection {
    pub metadata: AccountInfo,
    pub collection_authority: AccountInfo,
    pub payer: AccountInfo,
    pub update_authority: AccountInfo,
    pub collection_mint: AccountInfo,
    pub collection_metadata: AccountInfo,
    pub collection_master_edition: AccountInfo,
}

#[derive(Accounts)]
pub struct SetAndVerifySizedCollectionItem {
    pub metadata: AccountInfo,
    pub collection_authority: AccountInfo,
    pub payer: AccountInfo,
    pub update_authority: AccountInfo,
    pub collection_mint: AccountInfo,
    pub collection_metadata: AccountInfo,
    pub collection_master_edition: AccountInfo,
}

#[derive(Accounts)]
pub struct FreezeDelegatedAccount {
    pub metadata: AccountInfo,
    pub delegate: AccountInfo,
    pub token_account: AccountInfo,
    pub edition: AccountInfo,
    pub mint: AccountInfo,
    pub token_program: AccountInfo,
}

#[derive(Accounts)]
pub struct ThawDelegatedAccount {
    pub metadata: AccountInfo,
    pub delegate: AccountInfo,
    pub token_account: AccountInfo,
    pub edition: AccountInfo,
    pub mint: AccountInfo,
    pub token_program: AccountInfo,
}

#[derive(Accounts)]
pub struct UpdatePrimarySaleHappenedViaToken {
    pub metadata: AccountInfo,
    pub owner: AccountInfo,
    pub token: AccountInfo,
}

#[derive(Accounts)]
pub struct SignMetadata {
    pub creator: AccountInfo,
    pub metadata: AccountInfo,
}

#[derive(Accounts)]
pub struct RemoveCreatorVerification {
    pub creator: AccountInfo,
    pub metadata: AccountInfo,
}

#[derive(Accounts)]
pub struct Utilize {
    pub metadata: AccountInfo,
    pub token_account: AccountInfo,
    pub mint: AccountInfo,
    pub use_authority: AccountInfo,
    pub owner: AccountInfo,
}

#[derive(Accounts)]
pub struct UnverifyCollection {
    pub metadata: AccountInfo,
    pub collection_authority: AccountInfo,
    pub collection_mint: AccountInfo,
    pub collection: AccountInfo,
    pub collection_master_edition_account: AccountInfo,
}

#[derive(Accounts)]
pub struct UnverifySizedCollectionItem {
    pub metadata: AccountInfo,
    pub collection_authority: AccountInfo,
    pub payer: AccountInfo,
    pub collection_mint: AccountInfo,
    pub collection: AccountInfo,
    pub collection_master_edition_account: AccountInfo,
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
