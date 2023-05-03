use anchor_lang::context::CpiContext;
use anchor_lang::error::ErrorCode;
use anchor_lang::{Accounts, Result, ToAccountInfos};
use mpl_token_metadata::state::{CollectionDetails, DataV2, TokenMetadataAccount};
use mpl_token_metadata::ID;
use solana_program::account_info::AccountInfo;
use solana_program::pubkey::Pubkey;
use std::ops::Deref;

pub fn approve_collection_authority<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, ApproveCollectionAuthority<'info>>,
) -> Result<()> {
    let ix = mpl_token_metadata::instruction::approve_collection_authority(
        ID,
        *ctx.accounts.collection_authority_record.key,
        *ctx.accounts.new_collection_authority.key,
        *ctx.accounts.update_authority.key,
        *ctx.accounts.payer.key,
        *ctx.accounts.metadata.key,
        *ctx.accounts.mint.key,
    );
    solana_program::program::invoke_signed(
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
    let ix = mpl_token_metadata::instruction::bubblegum_set_collection_size(
        ID,
        *ctx.accounts.metadata_account.key,
        *ctx.accounts.update_authority.key,
        *ctx.accounts.mint.key,
        *ctx.accounts.bubblegum_signer.key,
        collection_authority_record,
        size,
    );
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn burn_edition_nft<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, BurnEditionNft<'info>>,
) -> Result<()> {
    let ix = mpl_token_metadata::instruction::burn_edition_nft(
        ID,
        *ctx.accounts.metadata.key,
        *ctx.accounts.owner.key,
        *ctx.accounts.print_edition_mint.key,
        *ctx.accounts.master_edition_mint.key,
        *ctx.accounts.print_edition_token.key,
        *ctx.accounts.master_edition_token.key,
        *ctx.accounts.master_edition.key,
        *ctx.accounts.print_edition.key,
        *ctx.accounts.edition_marker.key,
        *ctx.accounts.spl_token.key,
    );
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn burn_nft<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, BurnNft<'info>>,
    collection_metadata: Option<Pubkey>,
) -> Result<()> {
    let ix = mpl_token_metadata::instruction::burn_nft(
        ID,
        *ctx.accounts.metadata.key,
        *ctx.accounts.owner.key,
        *ctx.accounts.mint.key,
        *ctx.accounts.token.key,
        *ctx.accounts.edition.key,
        *ctx.accounts.spl_token.key,
        collection_metadata,
    );
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn create_metadata_accounts_v3<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, CreateMetadataAccountsV3<'info>>,
    data: DataV2,
    is_mutable: bool,
    update_authority_is_signer: bool,
    details: Option<CollectionDetails>,
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
    let ix = mpl_token_metadata::instruction::create_metadata_accounts_v3(
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
        details,
    );
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
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
    )
    .map_err(Into::into)
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
    )
    .map_err(Into::into)
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
    )
    .map_err(Into::into)
}

pub fn revoke_collection_authority<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, RevokeCollectionAuthority<'info>>,
) -> Result<()> {
    let ix = mpl_token_metadata::instruction::revoke_collection_authority(
        ID,
        *ctx.accounts.collection_authority_record.key,
        *ctx.accounts.delegate_authority.key,
        *ctx.accounts.revoke_authority.key,
        *ctx.accounts.metadata.key,
        *ctx.accounts.mint.key,
    );
    solana_program::program::invoke_signed(
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
    let ix = mpl_token_metadata::instruction::set_collection_size(
        ID,
        *ctx.accounts.metadata.key,
        *ctx.accounts.update_authority.key,
        *ctx.accounts.mint.key,
        collection_authority_record,
        size,
    );
    solana_program::program::invoke_signed(
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
    let ix = mpl_token_metadata::instruction::verify_collection(
        ID,
        *ctx.accounts.metadata.key,
        *ctx.accounts.collection_authority.key,
        *ctx.accounts.payer.key,
        *ctx.accounts.collection_mint.key,
        *ctx.accounts.collection_metadata.key,
        *ctx.accounts.collection_master_edition.key,
        collection_authority_record,
    );
    solana_program::program::invoke_signed(
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
    let ix = mpl_token_metadata::instruction::verify_sized_collection_item(
        ID,
        *ctx.accounts.metadata.key,
        *ctx.accounts.collection_authority.key,
        *ctx.accounts.payer.key,
        *ctx.accounts.collection_mint.key,
        *ctx.accounts.collection_metadata.key,
        *ctx.accounts.collection_master_edition.key,
        collection_authority_record,
    );
    solana_program::program::invoke_signed(
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
    let ix = mpl_token_metadata::instruction::set_and_verify_collection(
        ID,
        *ctx.accounts.metadata.key,
        *ctx.accounts.collection_authority.key,
        *ctx.accounts.payer.key,
        *ctx.accounts.update_authority.key,
        *ctx.accounts.collection_mint.key,
        *ctx.accounts.collection_metadata.key,
        *ctx.accounts.collection_master_edition.key,
        collection_authority_record,
    );
    solana_program::program::invoke_signed(
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
    let ix = mpl_token_metadata::instruction::set_and_verify_sized_collection_item(
        ID,
        *ctx.accounts.metadata.key,
        *ctx.accounts.collection_authority.key,
        *ctx.accounts.payer.key,
        *ctx.accounts.update_authority.key,
        *ctx.accounts.collection_mint.key,
        *ctx.accounts.collection_metadata.key,
        *ctx.accounts.collection_master_edition.key,
        collection_authority_record,
    );
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn freeze_delegated_account<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, FreezeDelegatedAccount<'info>>,
) -> Result<()> {
    let ix = mpl_token_metadata::instruction::freeze_delegated_account(
        ID,
        *ctx.accounts.delegate.key,
        *ctx.accounts.token_account.key,
        *ctx.accounts.edition.key,
        *ctx.accounts.mint.key,
    );
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn thaw_delegated_account<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, ThawDelegatedAccount<'info>>,
) -> Result<()> {
    let ix = mpl_token_metadata::instruction::thaw_delegated_account(
        ID,
        *ctx.accounts.delegate.key,
        *ctx.accounts.token_account.key,
        *ctx.accounts.edition.key,
        *ctx.accounts.mint.key,
    );
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn update_primary_sale_happened_via_token<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, UpdatePrimarySaleHappenedViaToken<'info>>,
) -> Result<()> {
    let ix = mpl_token_metadata::instruction::update_primary_sale_happened_via_token(
        ID,
        *ctx.accounts.metadata.key,
        *ctx.accounts.owner.key,
        *ctx.accounts.token.key,
    );

    solana_program::program::invoke_signed(
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
    let ix = mpl_token_metadata::instruction::set_token_standard(
        ID,
        *ctx.accounts.metadata_account.key,
        *ctx.accounts.update_authority.key,
        *ctx.accounts.mint_account.key,
        edition_account,
    );
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn sign_metadata<'info>(ctx: CpiContext<'_, '_, '_, 'info, SignMetadata<'info>>) -> Result<()> {
    let ix = mpl_token_metadata::instruction::sign_metadata(
        ID,
        *ctx.accounts.metadata.key,
        *ctx.accounts.creator.key,
    );

    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;
    Ok(())
}

pub fn remove_creator_verification<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, RemoveCreatorVerification<'info>>,
) -> Result<()> {
    let ix = mpl_token_metadata::instruction::remove_creator_verification(
        ID,
        *ctx.accounts.metadata.key,
        *ctx.accounts.creator.key,
    );
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;
    Ok(())
}

pub fn utilize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, Utilize<'info>>,
    use_authority_record_pda: Option<Pubkey>,
    burner: Option<Pubkey>,
    number_of_uses: u64,
) -> Result<()> {
    let ix = mpl_token_metadata::instruction::utilize(
        ID,
        *ctx.accounts.metadata.key,
        *ctx.accounts.token_account.key,
        *ctx.accounts.mint.key,
        use_authority_record_pda,
        *ctx.accounts.use_authority.key,
        *ctx.accounts.owner.key,
        burner,
        number_of_uses,
    );
    solana_program::program::invoke_signed(
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
    let ix = mpl_token_metadata::instruction::unverify_collection(
        ID,
        *ctx.accounts.metadata.key,
        *ctx.accounts.collection_authority.key,
        *ctx.accounts.collection_mint.key,
        *ctx.accounts.collection.key,
        *ctx.accounts.collection_master_edition_account.key,
        collection_authority_record,
    );
    solana_program::program::invoke_signed(
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
    let ix = mpl_token_metadata::instruction::unverify_sized_collection_item(
        ID,
        *ctx.accounts.metadata.key,
        *ctx.accounts.collection_authority.key,
        *ctx.accounts.payer.key,
        *ctx.accounts.collection_mint.key,
        *ctx.accounts.collection.key,
        *ctx.accounts.collection_master_edition_account.key,
        collection_authority_record,
    );
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct ApproveCollectionAuthority<'info> {
    pub collection_authority_record: AccountInfo<'info>,
    pub new_collection_authority: AccountInfo<'info>,
    pub update_authority: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub metadata: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct BubblegumSetCollectionSize<'info> {
    pub metadata_account: AccountInfo<'info>,
    pub update_authority: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub bubblegum_signer: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct BurnEditionNft<'info> {
    pub metadata: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,
    pub print_edition_mint: AccountInfo<'info>,
    pub master_edition_mint: AccountInfo<'info>,
    pub print_edition_token: AccountInfo<'info>,
    pub master_edition_token: AccountInfo<'info>,
    pub master_edition: AccountInfo<'info>,
    pub print_edition: AccountInfo<'info>,
    pub edition_marker: AccountInfo<'info>,
    pub spl_token: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct BurnNft<'info> {
    pub metadata: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub token: AccountInfo<'info>,
    pub edition: AccountInfo<'info>,
    pub spl_token: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CreateMetadataAccountsV3<'info> {
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

#[derive(Accounts)]
pub struct RevokeCollectionAuthority<'info> {
    pub collection_authority_record: AccountInfo<'info>,
    pub delegate_authority: AccountInfo<'info>,
    pub revoke_authority: AccountInfo<'info>,
    pub metadata: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SetCollectionSize<'info> {
    pub metadata: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub update_authority: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SetTokenStandard<'info> {
    pub metadata_account: AccountInfo<'info>,
    pub update_authority: AccountInfo<'info>,
    pub mint_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct VerifyCollection<'info> {
    pub payer: AccountInfo<'info>,
    pub metadata: AccountInfo<'info>,
    pub collection_authority: AccountInfo<'info>,
    pub collection_mint: AccountInfo<'info>,
    pub collection_metadata: AccountInfo<'info>,
    pub collection_master_edition: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct VerifySizedCollectionItem<'info> {
    pub payer: AccountInfo<'info>,
    pub metadata: AccountInfo<'info>,
    pub collection_authority: AccountInfo<'info>,
    pub collection_mint: AccountInfo<'info>,
    pub collection_metadata: AccountInfo<'info>,
    pub collection_master_edition: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SetAndVerifyCollection<'info> {
    pub metadata: AccountInfo<'info>,
    pub collection_authority: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub update_authority: AccountInfo<'info>,
    pub collection_mint: AccountInfo<'info>,
    pub collection_metadata: AccountInfo<'info>,
    pub collection_master_edition: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SetAndVerifySizedCollectionItem<'info> {
    pub metadata: AccountInfo<'info>,
    pub collection_authority: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub update_authority: AccountInfo<'info>,
    pub collection_mint: AccountInfo<'info>,
    pub collection_metadata: AccountInfo<'info>,
    pub collection_master_edition: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct FreezeDelegatedAccount<'info> {
    pub metadata: AccountInfo<'info>,
    pub delegate: AccountInfo<'info>,
    pub token_account: AccountInfo<'info>,
    pub edition: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct ThawDelegatedAccount<'info> {
    pub metadata: AccountInfo<'info>,
    pub delegate: AccountInfo<'info>,
    pub token_account: AccountInfo<'info>,
    pub edition: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct UpdatePrimarySaleHappenedViaToken<'info> {
    pub metadata: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,
    pub token: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SignMetadata<'info> {
    pub creator: AccountInfo<'info>,
    pub metadata: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct RemoveCreatorVerification<'info> {
    pub creator: AccountInfo<'info>,
    pub metadata: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Utilize<'info> {
    pub metadata: AccountInfo<'info>,
    pub token_account: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub use_authority: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct UnverifyCollection<'info> {
    pub metadata: AccountInfo<'info>,
    pub collection_authority: AccountInfo<'info>,
    pub collection_mint: AccountInfo<'info>,
    pub collection: AccountInfo<'info>,
    pub collection_master_edition_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct UnverifySizedCollectionItem<'info> {
    pub metadata: AccountInfo<'info>,
    pub collection_authority: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub collection_mint: AccountInfo<'info>,
    pub collection: AccountInfo<'info>,
    pub collection_master_edition_account: AccountInfo<'info>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetadataAccount(mpl_token_metadata::state::Metadata);

impl MetadataAccount {
    pub const LEN: usize = mpl_token_metadata::state::MAX_METADATA_LEN;
}

impl anchor_lang::AccountDeserialize for MetadataAccount {
    fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        let md = Self::try_deserialize_unchecked(buf)?;
        if md.key != mpl_token_metadata::state::Metadata::key() {
            return Err(ErrorCode::AccountNotInitialized.into());
        }
        Ok(md)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        let md = mpl_token_metadata::state::Metadata::safe_deserialize(buf)?;
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
    type Target = mpl_token_metadata::state::Metadata;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MasterEditionAccount(mpl_token_metadata::state::MasterEditionV2);

impl MasterEditionAccount {
    pub const LEN: usize = mpl_token_metadata::state::MAX_MASTER_EDITION_LEN;
}

impl anchor_lang::AccountDeserialize for MasterEditionAccount {
    fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        let me = Self::try_deserialize_unchecked(buf)?;
        if me.key != mpl_token_metadata::state::MasterEditionV2::key() {
            return Err(ErrorCode::AccountNotInitialized.into());
        }
        Ok(me)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        let result = mpl_token_metadata::state::MasterEditionV2::safe_deserialize(buf)?;
        Ok(Self(result))
    }
}

impl Deref for MasterEditionAccount {
    type Target = mpl_token_metadata::state::MasterEditionV2;
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

#[derive(Clone)]
pub struct Metadata;

impl anchor_lang::Id for Metadata {
    fn id() -> Pubkey {
        ID
    }
}
