// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use {
    anchor_lang::{
        context::CpiContext,
        pinocchio_runtime::{account_info::AccountInfo, program_pack::Pack, pubkey::Pubkey},
        Accounts, Result,
    },
    std::ops::Deref,
};
pub use {spl_token::ID, spl_token_interface as spl_token};

pub fn transfer(ctx: CpiContext<'_, '_, Transfer>, amount: u64) -> Result<()> {
    let ix = spl_token::instruction::transfer(
        &spl_token::ID,
        ctx.accounts.from.address(),
        ctx.accounts.to.address(),
        ctx.accounts.authority.address(),
        &[],
        amount,
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[ctx.accounts.from, ctx.accounts.to, ctx.accounts.authority],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn transfer_checked(
    ctx: CpiContext<'_, '_, TransferChecked>,
    amount: u64,
    decimals: u8,
) -> Result<()> {
    let ix = spl_token::instruction::transfer_checked(
        &spl_token::ID,
        ctx.accounts.from.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.to.address(),
        ctx.accounts.authority.address(),
        &[],
        amount,
        decimals,
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[
            ctx.accounts.from,
            ctx.accounts.mint,
            ctx.accounts.to,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn mint_to(ctx: CpiContext<'_, '_, MintTo>, amount: u64) -> Result<()> {
    let ix = spl_token::instruction::mint_to(
        &spl_token::ID,
        ctx.accounts.mint.address(),
        ctx.accounts.to.address(),
        ctx.accounts.authority.address(),
        &[],
        amount,
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[ctx.accounts.to, ctx.accounts.mint, ctx.accounts.authority],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn burn(ctx: CpiContext<'_, '_, Burn>, amount: u64) -> Result<()> {
    let ix = spl_token::instruction::burn(
        &spl_token::ID,
        ctx.accounts.from.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.authority.address(),
        &[],
        amount,
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[ctx.accounts.from, ctx.accounts.mint, ctx.accounts.authority],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn approve(ctx: CpiContext<'_, '_, Approve>, amount: u64) -> Result<()> {
    let ix = spl_token::instruction::approve(
        &spl_token::ID,
        ctx.accounts.to.address(),
        ctx.accounts.delegate.address(),
        ctx.accounts.authority.address(),
        &[],
        amount,
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[
            ctx.accounts.to,
            ctx.accounts.delegate,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn approve_checked(
    ctx: CpiContext<'_, '_, ApproveChecked>,
    amount: u64,
    decimals: u8,
) -> Result<()> {
    let ix = spl_token::instruction::approve_checked(
        &spl_token::ID,
        ctx.accounts.to.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.delegate.address(),
        ctx.accounts.authority.address(),
        &[],
        amount,
        decimals,
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[
            ctx.accounts.to,
            ctx.accounts.mint,
            ctx.accounts.delegate,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn revoke(ctx: CpiContext<'_, '_, Revoke>) -> Result<()> {
    let ix = spl_token::instruction::revoke(
        &spl_token::ID,
        ctx.accounts.source.address(),
        ctx.accounts.authority.address(),
        &[],
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[ctx.accounts.source, ctx.accounts.authority],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn initialize_account(ctx: CpiContext<'_, '_, InitializeAccount>) -> Result<()> {
    let ix = spl_token::instruction::initialize_account(
        &spl_token::ID,
        ctx.accounts.account.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.authority.address(),
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[
            ctx.accounts.account,
            ctx.accounts.mint,
            ctx.accounts.authority,
            ctx.accounts.rent,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn initialize_account3(ctx: CpiContext<'_, '_, InitializeAccount3>) -> Result<()> {
    let ix = spl_token::instruction::initialize_account3(
        &spl_token::ID,
        ctx.accounts.account.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.authority.address(),
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[ctx.accounts.account, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn close_account(ctx: CpiContext<'_, '_, CloseAccount>) -> Result<()> {
    let ix = spl_token::instruction::close_account(
        &spl_token::ID,
        ctx.accounts.account.address(),
        ctx.accounts.destination.address(),
        ctx.accounts.authority.address(),
        &[], // TODO: support multisig
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[
            ctx.accounts.account,
            ctx.accounts.destination,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn freeze_account(ctx: CpiContext<'_, '_, FreezeAccount>) -> Result<()> {
    let ix = spl_token::instruction::freeze_account(
        &spl_token::ID,
        ctx.accounts.account.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.authority.address(),
        &[], // TODO: Support multisig signers.
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[
            ctx.accounts.account,
            ctx.accounts.mint,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn thaw_account(ctx: CpiContext<'_, '_, ThawAccount>) -> Result<()> {
    let ix = spl_token::instruction::thaw_account(
        &spl_token::ID,
        ctx.accounts.account.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.authority.address(),
        &[], // TODO: Support multisig signers.
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[
            ctx.accounts.account,
            ctx.accounts.mint,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn initialize_mint(
    ctx: CpiContext<'_, '_, InitializeMint>,
    decimals: u8,
    authority: &Pubkey,
    freeze_authority: Option<&Pubkey>,
) -> Result<()> {
    let ix = spl_token::instruction::initialize_mint(
        &spl_token::ID,
        ctx.accounts.mint.address(),
        authority,
        freeze_authority,
        decimals,
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[ctx.accounts.mint, ctx.accounts.rent],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn initialize_mint2(
    ctx: CpiContext<'_, '_, InitializeMint2>,
    decimals: u8,
    authority: &Pubkey,
    freeze_authority: Option<&Pubkey>,
) -> Result<()> {
    let ix = spl_token::instruction::initialize_mint2(
        &spl_token::ID,
        ctx.accounts.mint.address(),
        authority,
        freeze_authority,
        decimals,
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(ix, &[ctx.accounts.mint], ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn set_authority(
    ctx: CpiContext<'_, '_, SetAuthority>,
    authority_type: spl_token::instruction::AuthorityType,
    new_authority: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token::instruction::set_authority(
        &spl_token::ID,
        ctx.accounts.account_or_mint.address(),
        new_authority.as_ref(),
        authority_type,
        ctx.accounts.current_authority.address(),
        &[], // TODO: Support multisig signers.
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[ctx.accounts.account_or_mint, ctx.accounts.current_authority],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn sync_native(ctx: CpiContext<'_, '_, SyncNative>) -> Result<()> {
    let ix = spl_token::instruction::sync_native(&spl_token::ID, ctx.accounts.account.address())?;
    crate::cpi_util::invoke_signed_solana_instruction(ix, &[ctx.accounts.account], ctx.signer_seeds)
        .map_err(Into::into)
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

#[derive(Clone, Debug, Default, PartialEq, Copy)]
pub struct TokenAccount(spl_token::state::Account);

impl TokenAccount {
    pub const LEN: usize = spl_token::state::Account::LEN;
}

impl anchor_lang::AccountDeserialize for TokenAccount {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        spl_token::state::Account::unpack(buf)
            .map(TokenAccount)
            .map_err(Into::into)
    }
}

impl anchor_lang::AccountSerialize for TokenAccount {}

impl anchor_lang::Owner for TokenAccount {
    fn owner() -> Pubkey {
        ID
    }
}

impl Deref for TokenAccount {
    type Target = spl_token::state::Account;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, Default, PartialEq, Copy)]
pub struct Mint(spl_token::state::Mint);

impl Mint {
    pub const LEN: usize = spl_token::state::Mint::LEN;
}

impl anchor_lang::AccountDeserialize for Mint {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        spl_token::state::Mint::unpack(buf)
            .map(Mint)
            .map_err(Into::into)
    }
}

impl anchor_lang::AccountSerialize for Mint {}

impl anchor_lang::Owner for Mint {
    fn owner() -> Pubkey {
        ID
    }
}

impl Deref for Mint {
    type Target = spl_token::state::Mint;

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
