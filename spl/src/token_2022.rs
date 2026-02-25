// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};
use pinocchio_token_2022::ID;

#[deprecated(
    since = "0.28.0",
    note = "please use `transfer_checked` or `transfer_checked_with_fee` instead"
)]
pub fn transfer(ctx: CpiContext<'_, '_, Transfer>, amount: u64) -> Result<()> {
    #[allow(deprecated)]
    let ix = pinocchio_token_2022::instructions::Transfer {
        from: &ctx.accounts.from,
        to: &ctx.accounts.to,
        authority: &ctx.accounts.authority,
        amount,
        token_program: &ctx.program_id,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn transfer_checked(
    ctx: CpiContext<'_, '_, TransferChecked>,
    amount: u64,
    decimals: u8,
) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::TransferChecked {
        from: &ctx.accounts.from,
        mint: &ctx.accounts.mint,
        to: &ctx.accounts.to,
        authority: &ctx.accounts.authority,
        amount,
        decimals,
        token_program: &ctx.program_id,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn mint_to(ctx: CpiContext<'_, '_, MintTo>, amount: u64) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::MintTo {
        mint: &ctx.accounts.mint,
        amount,
        token_program: &ctx.program_id,
        account: &ctx.accounts.to,
        mint_authority: &ctx.accounts.authority,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn mint_to_checked(
    ctx: CpiContext<'_, '_, MintToChecked>,
    amount: u64,
    decimals: u8,
) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::MintToChecked {
        mint: &ctx.accounts.mint,
        account: &ctx.accounts.to,
        mint_authority: &ctx.accounts.authority,
        amount,
        decimals,
        token_program: &ctx.program_id,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn burn(ctx: CpiContext<'_, '_, Burn>, amount: u64) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::Burn {
        mint: &ctx.accounts.mint,
        account: &ctx.accounts.from,
        authority: &ctx.accounts.authority,
        amount,
        token_program: &ctx.program_id,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn burn_checked(ctx: CpiContext<'_, '_, BurnChecked>, amount: u64, decimals: u8) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::BurnChecked {
        mint: &ctx.accounts.mint,
        account: &ctx.accounts.from,
        authority: &ctx.accounts.authority,
        amount,
        decimals,
        token_program: &ctx.program_id,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn approve(ctx: CpiContext<'_, '_, Approve>, amount: u64) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::Approve {
        source: &ctx.accounts.to,
        delegate: &ctx.accounts.delegate,
        authority: &ctx.accounts.authority,
        amount,
        token_program: &ctx.program_id,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn approve_checked(
    ctx: CpiContext<'_, '_, ApproveChecked>,
    amount: u64,
    decimals: u8,
) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::ApproveChecked {
        source: &ctx.accounts.to,
        mint: &ctx.accounts.mint,
        delegate: &ctx.accounts.delegate,
        authority: &ctx.accounts.authority,
        amount,
        decimals,
        token_program: &ctx.program_id,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn revoke(ctx: CpiContext<'_, '_, Revoke>) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::Revoke {
        source: &ctx.accounts.source,
        authority: &ctx.accounts.authority,
        token_program: &ctx.program_id,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn initialize_account(ctx: CpiContext<'_, '_, InitializeAccount>) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::InitializeAccount {
        account: &ctx.accounts.account,
        mint: &ctx.accounts.mint,
        owner: &ctx.accounts.authority,
        token_program: &ctx.program_id,
        rent_sysvar: &ctx.accounts.rent,
    };
    ix.invoke().map_err(Into::into)
}

pub fn initialize_account3(ctx: CpiContext<'_, '_, InitializeAccount3>) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::InitializeAccount3 {
        account: &ctx.accounts.account,
        mint: &ctx.accounts.mint,
        owner: &ctx.accounts.authority.key(),
        token_program: &ctx.program_id,
    };
    ix.invoke().map_err(Into::into)
}

pub fn initialize_non_transferable_mint(
    ctx: CpiContext<'_, '_, InitializeNonTransferableMint>,
) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::InitializeNonTransferableMint {
        mint: &ctx.accounts.mint,
        token_program: &ctx.program_id,
    };
    ix.invoke().map_err(Into::into)
}

pub fn close_account(ctx: CpiContext<'_, '_, CloseAccount>) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::CloseAccount {
        account: &ctx.accounts.account,
        destination: &ctx.accounts.destination,
        authority: &ctx.accounts.authority,
        token_program: &ctx.program_id,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn freeze_account(ctx: CpiContext<'_, '_, FreezeAccount>) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::FreezeAccount {
        account: &ctx.accounts.account,
        mint: &ctx.accounts.mint,
        freeze_authority: &ctx.accounts.authority,
        token_program: &ctx.program_id,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn thaw_account(ctx: CpiContext<'_, '_, ThawAccount>) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::ThawAccount {
        account: &ctx.accounts.account,
        mint: &ctx.accounts.mint,
        freeze_authority: &ctx.accounts.authority,
        token_program: &ctx.program_id,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn initialize_mint(
    ctx: CpiContext<'_, '_, InitializeMint>,
    decimals: u8,
    authority: &Pubkey,
    freeze_authority: Option<&Pubkey>,
) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::InitializeMint {
        mint: &ctx.accounts.mint,
        freeze_authority,
        decimals,
        token_program: &ctx.program_id,
        rent_sysvar: &ctx.accounts.rent,
        mint_authority: authority,
    };
    ix.invoke().map_err(Into::into)
}

pub fn initialize_mint2(
    ctx: CpiContext<'_, '_, InitializeMint2>,
    decimals: u8,
    authority: &Pubkey,
    freeze_authority: Option<&Pubkey>,
) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::InitializeMint2 {
        mint: &ctx.accounts.mint,
        freeze_authority,
        decimals,
        token_program: &ctx.program_id,
        mint_authority: authority,
    };
    ix.invoke().map_err(Into::into)
}

pub fn set_authority(
    ctx: CpiContext<'_, '_, SetAuthority>,
    authority_type: pinocchio_token_2022::instructions::AuthorityType,
    new_authority: Option<&Pubkey>,
) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::SetAuthority {
        account: &ctx.accounts.account_or_mint,
        authority: &ctx.accounts.current_authority,
        authority_type,
        new_authority,
        token_program: &ctx.program_id,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn sync_native(ctx: CpiContext<'_, '_, SyncNative>) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::SyncNative {
        native_token: &ctx.accounts.account,
        token_program: &ctx.program_id,
    };
    ix.invoke().map_err(Into::into)
}

pub fn unwrap_lamports(ctx: CpiContext<'_, '_, UnwrapLamports>, amount: Option<u64>) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::UnwrapLamports {
        source: &ctx.accounts.account,
        destination: &ctx.accounts.destination,
        authority: &ctx.accounts.authority,
        multisig_signers: &ctx.remaining_accounts.iter().collect::<Vec<&AccountInfo>>(),
        amount,
        token_program: &ctx.program_id,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct Transfer {
    pub from: AccountInfo,
    pub to: AccountInfo,
    pub authority: AccountInfo,
}

#[derive(Accounts)]
pub struct TransferChecked {
    pub from: AccountInfo,
    pub mint: AccountInfo,
    pub to: AccountInfo,
    pub authority: AccountInfo,
}

#[derive(Accounts)]
pub struct MintTo {
    pub mint: AccountInfo,
    pub to: AccountInfo,
    pub authority: AccountInfo,
}

#[derive(Accounts)]
pub struct MintToChecked {
    pub mint: AccountInfo,
    pub to: AccountInfo,
    pub authority: AccountInfo,
}

#[derive(Accounts)]
pub struct Burn {
    pub mint: AccountInfo,
    pub from: AccountInfo,
    pub authority: AccountInfo,
}

#[derive(Accounts)]
pub struct BurnChecked {
    pub mint: AccountInfo,
    pub from: AccountInfo,
    pub authority: AccountInfo,
}

#[derive(Accounts)]
pub struct Approve {
    pub to: AccountInfo,
    pub delegate: AccountInfo,
    pub authority: AccountInfo,
}

#[derive(Accounts)]
pub struct ApproveChecked {
    pub to: AccountInfo,
    pub mint: AccountInfo,
    pub delegate: AccountInfo,
    pub authority: AccountInfo,
}

#[derive(Accounts)]
pub struct Revoke {
    pub source: AccountInfo,
    pub authority: AccountInfo,
}

#[derive(Accounts)]
pub struct InitializeAccount {
    pub account: AccountInfo,
    pub mint: AccountInfo,
    pub authority: AccountInfo,
    pub rent: AccountInfo,
}

#[derive(Accounts)]
pub struct InitializeAccount3 {
    pub account: AccountInfo,
    pub mint: AccountInfo,
    pub authority: AccountInfo,
}

#[derive(Accounts)]
pub struct CloseAccount {
    pub account: AccountInfo,
    pub destination: AccountInfo,
    pub authority: AccountInfo,
}

#[derive(Accounts)]
pub struct FreezeAccount {
    pub account: AccountInfo,
    pub mint: AccountInfo,
    pub authority: AccountInfo,
}

#[derive(Accounts)]
pub struct ThawAccount {
    pub account: AccountInfo,
    pub mint: AccountInfo,
    pub authority: AccountInfo,
}

#[derive(Accounts)]
pub struct InitializeMint {
    pub mint: AccountInfo,
    pub rent: AccountInfo,
}

#[derive(Accounts)]
pub struct InitializeMint2 {
    pub mint: AccountInfo,
}

#[derive(Accounts)]
pub struct InitializeNonTransferableMint {
    pub mint: AccountInfo,
}

#[derive(Accounts)]
pub struct SetAuthority {
    pub current_authority: AccountInfo,
    pub account_or_mint: AccountInfo,
}

#[derive(Accounts)]
pub struct SyncNative {
    pub account: AccountInfo,
}

#[derive(Accounts)]
pub struct UnwrapLamports {
    pub account: AccountInfo,
    pub destination: AccountInfo,
    pub authority: AccountInfo,
}

#[derive(Clone)]
pub struct Token2022;

impl anchor_lang::Id for Token2022 {
    fn id() -> Pubkey {
        ID
    }
}
