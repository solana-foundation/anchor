// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};
use std::ops::Deref;

use pinocchio_token::ID;

pub fn transfer(ctx: CpiContext<'_, '_, Transfer>, amount: u64) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token::instructions::Transfer {
        from: &ctx.accounts.from,
        to: &ctx.accounts.to,
        authority: &ctx.accounts.authority,
        amount,
        multisig_signers: &signers,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn transfer_checked(
    ctx: CpiContext<'_, '_, TransferChecked>,
    amount: u64,
    decimals: u8,
) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token::instructions::TransferChecked {
        from: &ctx.accounts.from,
        mint: &ctx.accounts.mint,
        to: &ctx.accounts.to,
        authority: &ctx.accounts.authority,
        amount,
        decimals,
        multisig_signers: &signers,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn mint_to(ctx: CpiContext<'_, '_, MintTo>, amount: u64) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token::instructions::MintTo {
        mint: &ctx.accounts.mint,
        account: &ctx.accounts.to,
        mint_authority: &ctx.accounts.authority,
        amount,
        multisig_signers: &signers,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn burn(ctx: CpiContext<'_, '_, Burn>, amount: u64) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token::instructions::Burn {
        account: &ctx.accounts.from,
        mint: &ctx.accounts.mint,
        authority: &ctx.accounts.authority,
        amount,
        multisig_signers: &signers,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn burn_checked(ctx: CpiContext<'_, '_, BurnChecked>, amount: u64, decimals: u8) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token::instructions::BurnChecked {
        account: &ctx.accounts.from,
        mint: &ctx.accounts.mint,
        authority: &ctx.accounts.authority,
        amount,
        decimals,
        multisig_signers: &signers,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn approve(ctx: CpiContext<'_, '_, Approve>, amount: u64) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token::instructions::Approve {
        source: &ctx.accounts.to,
        delegate: &ctx.accounts.delegate,
        authority: &ctx.accounts.authority,
        amount,
        multisig_signers: &signers,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn approve_checked(
    ctx: CpiContext<'_, '_, ApproveChecked>,
    amount: u64,
    decimals: u8,
) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token::instructions::ApproveChecked {
        source: &ctx.accounts.to,
        mint: &ctx.accounts.mint,
        delegate: &ctx.accounts.delegate,
        authority: &ctx.accounts.authority,
        amount,
        decimals,
        multisig_signers: &signers,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn revoke(ctx: CpiContext<'_, '_, Revoke>) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token::instructions::Revoke {
        source: &ctx.accounts.source,
        authority: &ctx.accounts.authority,
        multisig_signers: &signers,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn initialize_account(ctx: CpiContext<'_, '_, InitializeAccount>) -> Result<()> {
    let ix = pinocchio_token::instructions::InitializeAccount {
        account: &ctx.accounts.account,
        mint: &ctx.accounts.mint,
        owner: &ctx.accounts.authority,
        rent_sysvar: &ctx.accounts.rent,
    };
    ix.invoke().map_err(Into::into)
}

pub fn initialize_account3(ctx: CpiContext<'_, '_, InitializeAccount3>) -> Result<()> {
    let ix = pinocchio_token::instructions::InitializeAccount3 {
        account: &ctx.accounts.account,
        mint: &ctx.accounts.mint,
        owner: &ctx.accounts.authority.key(),
    };
    ix.invoke().map_err(Into::into)
}

pub fn close_account(ctx: CpiContext<'_, '_, CloseAccount>) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token::instructions::CloseAccount {
        account: &ctx.accounts.account,
        destination: &ctx.accounts.destination,
        authority: &ctx.accounts.authority,
        multisig_signers: &signers,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn freeze_account(ctx: CpiContext<'_, '_, FreezeAccount>) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token::instructions::FreezeAccount {
        account: &ctx.accounts.account,
        mint: &ctx.accounts.mint,
        freeze_authority: &ctx.accounts.authority,
        multisig_signers: &signers,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn thaw_account(ctx: CpiContext<'_, '_, ThawAccount>) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token::instructions::ThawAccount {
        account: &ctx.accounts.account,
        mint: &ctx.accounts.mint,
        freeze_authority: &ctx.accounts.authority,
        multisig_signers: &signers,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn initialize_mint(
    ctx: CpiContext<'_, '_, InitializeMint>,
    decimals: u8,
    authority: &Pubkey,
    freeze_authority: Option<&Pubkey>,
) -> Result<()> {
    let ix = pinocchio_token::instructions::InitializeMint {
        mint: &ctx.accounts.mint,
        rent_sysvar: &ctx.accounts.rent,
        decimals,
        mint_authority: authority,
        freeze_authority,
    };
    ix.invoke().map_err(Into::into)
}

pub fn initialize_mint2(
    ctx: CpiContext<'_, '_, InitializeMint2>,
    decimals: u8,
    authority: &Pubkey,
    freeze_authority: Option<&Pubkey>,
) -> Result<()> {
    let ix = pinocchio_token::instructions::InitializeMint2 {
        mint: &ctx.accounts.mint,
        decimals,
        mint_authority: authority,
        freeze_authority,
    };
    ix.invoke().map_err(Into::into)
}

pub fn set_authority(
    ctx: CpiContext<'_, '_, SetAuthority>,
    authority_type: pinocchio_token::instructions::AuthorityType,
    new_authority: Option<Pubkey>,
) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token::instructions::SetAuthority {
        account: &ctx.accounts.account_or_mint,
        authority: &ctx.accounts.current_authority,
        authority_type,
        new_authority: new_authority.as_ref(),
        multisig_signers: &signers,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn sync_native(ctx: CpiContext<'_, '_, SyncNative>) -> Result<()> {
    let ix = pinocchio_token::instructions::SyncNative {
        native_token: &ctx.accounts.account,
    };
    ix.invoke().map_err(Into::into)
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
pub struct SetAuthority {
    pub current_authority: AccountInfo,
    pub account_or_mint: AccountInfo,
}

#[derive(Accounts)]
pub struct SyncNative {
    pub account: AccountInfo,
}

#[derive()]
pub struct TokenAccount(pinocchio_token::state::TokenAccount);

impl TokenAccount {
    pub const LEN: usize = pinocchio_token::state::TokenAccount::LEN;
}

impl anchor_lang::AccountDeserialize for TokenAccount {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        let token_account_ref =
            unsafe { pinocchio_token::state::TokenAccount::from_bytes_unchecked(buf) };
        let token_account = unsafe { std::ptr::read(token_account_ref) };
        Ok(TokenAccount(token_account))
    }
}

impl anchor_lang::AccountSerialize for TokenAccount {}

impl anchor_lang::Owner for TokenAccount {
    fn owner() -> Pubkey {
        ID
    }
}

impl Deref for TokenAccount {
    type Target = pinocchio_token::state::TokenAccount;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive()]
pub struct Mint(pinocchio_token::state::Mint);

impl Mint {
    pub const LEN: usize = pinocchio_token::state::Mint::LEN;
}

impl anchor_lang::AccountDeserialize for Mint {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        let mint_ref = unsafe { pinocchio_token::state::Mint::from_bytes_unchecked(buf) };
        let mint = unsafe { std::ptr::read(mint_ref) };
        Ok(Mint(mint))
    }
}

impl anchor_lang::AccountSerialize for Mint {}

impl anchor_lang::Owner for Mint {
    fn owner() -> Pubkey {
        ID
    }
}

impl Deref for Mint {
    type Target = pinocchio_token::state::Mint;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct Token;

impl anchor_lang::Id for Token {
    fn id() -> Pubkey {
        ID
    }
}

// Field parsers to save compute. All account validation is assumed to be done
// outside of these methods.
pub mod accessor {
    use super::*;

    pub fn amount(account: &AccountInfo) -> Result<u64> {
        let bytes = account.try_borrow()?;
        let mut amount_bytes = [0u8; 8];
        amount_bytes.copy_from_slice(&bytes[64..72]);
        Ok(u64::from_le_bytes(amount_bytes))
    }

    pub fn mint(account: &AccountInfo) -> Result<Pubkey> {
        let bytes = account.try_borrow()?;
        let mut mint_bytes = [0u8; 32];
        mint_bytes.copy_from_slice(&bytes[..32]);
        Ok(Pubkey::new_from_array(mint_bytes))
    }

    pub fn authority(account: &AccountInfo) -> Result<Pubkey> {
        let bytes = account.try_borrow()?;
        let mut owner_bytes = [0u8; 32];
        owner_bytes.copy_from_slice(&bytes[32..64]);
        Ok(Pubkey::new_from_array(owner_bytes))
    }
}
