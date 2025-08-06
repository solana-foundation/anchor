use anchor_lang::{prelude::*, solana_program::entrypoint::ProgramResult};

use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::spl_token_2022::extension::{
        group_member_pointer::GroupMemberPointer, metadata_pointer::MetadataPointer,
        mint_close_authority::MintCloseAuthority, permanent_delegate::PermanentDelegate,
        transfer_hook::TransferHook,
    },
    token_interface::{
        get_mint_extension_data, spl_token_metadata_interface::state::TokenMetadata,
        token_metadata_initialize, Mint, Token2022, TokenAccount, TokenMetadataInitialize,
    },
};
use spl_pod::optional_keys::OptionalNonZeroPubkey;

use crate::{
    get_meta_list_size, get_mint_extensible_extension_data,
    update_account_lamports_to_minimum_balance, META_LIST_ACCOUNT_SEED,
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct CreateMintAccountArgs {
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

#[derive(Accounts)]
#[instruction(args: CreateMintAccountArgs)]
pub struct CreateMintAccount<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    /// CHECK: can be any account
    pub authority: Signer<'info>,
    #[account()]
    /// CHECK: can be any account
    pub receiver: UncheckedAccount<'info>,
    #[account(
        init,
        signer,
        payer = payer,
        mint::token_program = token_program,
        mint::decimals = 0,
        mint::authority = authority,
        mint::freeze_authority = authority,
        extensions::metadata_pointer::authority = authority,
        extensions::metadata_pointer::metadata_address = mint,
        extensions::group_member_pointer::authority = authority,
        extensions::group_member_pointer::member_address = mint,
        extensions::transfer_hook::authority = authority,
        extensions::transfer_hook::program_id = crate::ID,
        extensions::close_authority::authority = authority,
        extensions::permanent_delegate::delegate = authority,
    )]
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        init,
        payer = payer,
        associated_token::token_program = token_program,
        associated_token::mint = mint,
        associated_token::authority = receiver,
    )]
    pub mint_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    /// CHECK: This account's data is a buffer of TLV data
    #[account(
        init,
        space = get_meta_list_size(None),
        seeds = [META_LIST_ACCOUNT_SEED, mint.key().as_ref()],
        bump,
        payer = payer,
    )]
    pub extra_metas_account: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token2022>,
}

impl<'info> CreateMintAccount<'info> {
    fn initialize_token_metadata(
        &self,
        name: String,
        symbol: String,
        uri: String,
    ) -> ProgramResult {
        let cpi_accounts = TokenMetadataInitialize {
            program_id: self.token_program.to_account_info(),
            mint: self.mint.to_account_info(),
            metadata: self.mint.to_account_info(), // metadata account is the mint, since data is stored in mint
            mint_authority: self.authority.to_account_info(),
            update_authority: self.authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(self.token_program.to_account_info(), cpi_accounts);
        token_metadata_initialize(cpi_ctx, name, symbol, uri)?;
        Ok(())
    }
}

pub fn handler(ctx: Context<CreateMintAccount>, args: CreateMintAccountArgs) -> Result<()> {
    ctx.accounts.initialize_token_metadata(
        args.name.clone(),
        args.symbol.clone(),
        args.uri.clone(),
    )?;
    ctx.accounts.mint.reload()?;
    let mint_data = &mut ctx.accounts.mint.to_account_info();
    let metadata = get_mint_extensible_extension_data::<TokenMetadata>(mint_data)?;
    assert_eq!(metadata.mint, ctx.accounts.mint.key());
    assert_eq!(metadata.name, args.name);
    assert_eq!(metadata.symbol, args.symbol);
    assert_eq!(metadata.uri, args.uri);
    let metadata_pointer = get_mint_extension_data::<MetadataPointer>(mint_data)?;
    let mint_key: Option<Pubkey> = Some(ctx.accounts.mint.key());
    let authority_key: Option<Pubkey> = Some(ctx.accounts.authority.key());
    assert_eq!(
        metadata_pointer.metadata_address,
        OptionalNonZeroPubkey::try_from(mint_key)?
    );
    assert_eq!(
        metadata_pointer.authority,
        OptionalNonZeroPubkey::try_from(authority_key)?
    );
    let permanent_delegate = get_mint_extension_data::<PermanentDelegate>(mint_data)?;
    assert_eq!(
        permanent_delegate.delegate,
        OptionalNonZeroPubkey::try_from(authority_key)?
    );
    let close_authority = get_mint_extension_data::<MintCloseAuthority>(mint_data)?;
    assert_eq!(
        close_authority.close_authority,
        OptionalNonZeroPubkey::try_from(authority_key)?
    );
    let transfer_hook = get_mint_extension_data::<TransferHook>(mint_data)?;
    let program_id: Option<Pubkey> = Some(ctx.program_id.key());
    assert_eq!(
        transfer_hook.authority,
        OptionalNonZeroPubkey::try_from(authority_key)?
    );
    assert_eq!(
        transfer_hook.program_id,
        OptionalNonZeroPubkey::try_from(program_id)?
    );
    let group_member_pointer = get_mint_extension_data::<GroupMemberPointer>(mint_data)?;
    assert_eq!(
        group_member_pointer.authority,
        OptionalNonZeroPubkey::try_from(authority_key)?
    );
    assert_eq!(
        group_member_pointer.member_address,
        OptionalNonZeroPubkey::try_from(mint_key)?
    );
    // transfer minimum rent to mint account
    update_account_lamports_to_minimum_balance(
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
    )?;

    Ok(())
}

#[derive(Accounts)]
#[instruction()]
pub struct CheckMintExtensionConstraints<'info> {
    #[account(mut)]
    /// CHECK: can be any account
    pub authority: Signer<'info>,
    #[account(
        extensions::metadata_pointer::authority = authority,
        extensions::metadata_pointer::metadata_address = mint,
        extensions::group_member_pointer::authority = authority,
        extensions::group_member_pointer::member_address = mint,
        extensions::transfer_hook::authority = authority,
        extensions::transfer_hook::program_id = crate::ID,
        extensions::close_authority::authority = authority,
        extensions::permanent_delegate::delegate = authority,
    )]
    pub mint: Box<InterfaceAccount<'info, Mint>>,
}
