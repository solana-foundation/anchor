use anchor_lang::solana_program;
use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_lang::solana_program::entrypoint::ProgramResult;
use anchor_lang::solana_program::program_error::ProgramError;
use anchor_lang::solana_program::program_pack::Pack;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{Accounts, CpiContext};
use std::io::Write;
use std::ops::Deref;

pub use spl_token::ID;

pub fn transfer<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, Transfer<'info>>,
    amount: u64,
) -> ProgramResult {
    let ix = spl_token::instruction::transfer(
        &spl_token::ID,
        ctx.accounts.from.key,
        ctx.accounts.to.key,
        ctx.accounts.authority.key,
        &[],
        amount,
    )?;
    solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.from.clone(),
            ctx.accounts.to.clone(),
            ctx.accounts.authority.clone(),
            ctx.program.clone(),
        ],
        ctx.signer_seeds,
    )
}

pub fn mint_to<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, MintTo<'info>>,
    amount: u64,
) -> ProgramResult {
    let ix = spl_token::instruction::mint_to(
        &spl_token::ID,
        ctx.accounts.mint.key,
        ctx.accounts.to.key,
        ctx.accounts.authority.key,
        &[],
        amount,
    )?;
    solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.to.clone(),
            ctx.accounts.mint.clone(),
            ctx.accounts.authority.clone(),
            ctx.program.clone(),
        ],
        ctx.signer_seeds,
    )
}

pub fn burn<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, Burn<'info>>,
    amount: u64,
) -> ProgramResult {
    let ix = spl_token::instruction::burn(
        &spl_token::ID,
        ctx.accounts.to.key,
        ctx.accounts.mint.key,
        ctx.accounts.authority.key,
        &[],
        amount,
    )?;
    solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.to.clone(),
            ctx.accounts.mint.clone(),
            ctx.accounts.authority.clone(),
            ctx.program.clone(),
        ],
        ctx.signer_seeds,
    )
}

pub fn approve<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, Approve<'info>>,
    amount: u64,
) -> ProgramResult {
    let ix = spl_token::instruction::approve(
        &spl_token::ID,
        ctx.accounts.to.key,
        ctx.accounts.delegate.key,
        ctx.accounts.authority.key,
        &[],
        amount,
    )?;
    solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.to.clone(),
            ctx.accounts.delegate.clone(),
            ctx.accounts.authority.clone(),
            ctx.program.clone(),
        ],
        ctx.signer_seeds,
    )
}

pub fn initialize_account<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, InitializeAccount<'info>>,
) -> ProgramResult {
    let ix = spl_token::instruction::initialize_account(
        &spl_token::ID,
        ctx.accounts.account.key,
        ctx.accounts.mint.key,
        ctx.accounts.authority.key,
    )?;
    solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.account.clone(),
            ctx.accounts.mint.clone(),
            ctx.accounts.authority.clone(),
            ctx.accounts.rent.clone(),
            ctx.program.clone(),
        ],
        ctx.signer_seeds,
    )
}

pub fn close_account<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, CloseAccount<'info>>,
) -> ProgramResult {
    let ix = spl_token::instruction::close_account(
        &spl_token::ID,
        ctx.accounts.account.key,
        ctx.accounts.destination.key,
        ctx.accounts.authority.key,
        &[], // TODO: support multisig
    )?;
    solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.account.clone(),
            ctx.accounts.destination.clone(),
            ctx.accounts.authority.clone(),
        ],
        ctx.signer_seeds,
    )
}

pub fn freeze_account<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, FreezeAccount<'info>>,
) -> ProgramResult {
    let ix = spl_token::instruction::freeze_account(
        &spl_token::ID,
        ctx.accounts.account.key,
        ctx.accounts.mint.key,
        ctx.accounts.authority.key,
        &[], // TODO: Support multisig signers.
    )?;
    solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.account.clone(),
            ctx.accounts.mint.clone(),
            ctx.accounts.authority.clone(),
        ],
        ctx.signer_seeds,
    )
}

pub fn thaw_account<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, ThawAccount<'info>>,
) -> ProgramResult {
    let ix = spl_token::instruction::thaw_account(
        &spl_token::ID,
        ctx.accounts.account.key,
        ctx.accounts.mint.key,
        ctx.accounts.authority.key,
        &[], // TODO: Support multisig signers.
    )?;
    solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.account.clone(),
            ctx.accounts.mint.clone(),
            ctx.accounts.authority.clone(),
        ],
        ctx.signer_seeds,
    )
}

pub fn initialize_mint<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, InitializeMint<'info>>,
    decimals: u8,
    authority: &Pubkey,
    freeze_authority: Option<&Pubkey>,
) -> ProgramResult {
    let ix = spl_token::instruction::initialize_mint(
        &spl_token::ID,
        ctx.accounts.mint.key,
        authority,
        freeze_authority,
        decimals,
    )?;
    solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.mint.clone(),
            ctx.accounts.rent.clone(),
            ctx.program.clone(),
        ],
        ctx.signer_seeds,
    )
}

pub fn set_authority<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, SetAuthority<'info>>,
    authority_type: spl_token::instruction::AuthorityType,
    new_authority: Option<Pubkey>,
) -> ProgramResult {
    let mut spl_new_authority: Option<&Pubkey> = None;
    if new_authority.is_some() {
        spl_new_authority = new_authority.as_ref()
    }

    let ix = spl_token::instruction::set_authority(
        &spl_token::ID,
        ctx.accounts.account_or_mint.key,
        spl_new_authority,
        authority_type,
        ctx.accounts.current_authority.key,
        &[], // TODO: Support multisig signers.
    )?;
    solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.account_or_mint.clone(),
            ctx.accounts.current_authority.clone(),
            ctx.program.clone(),
        ],
        ctx.signer_seeds,
    )
}

#[derive(Accounts)]
pub struct Transfer<'info> {
    pub from: AccountInfo<'info>,
    pub to: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct MintTo<'info> {
    pub mint: AccountInfo<'info>,
    pub to: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Burn<'info> {
    pub mint: AccountInfo<'info>,
    pub to: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Approve<'info> {
    pub to: AccountInfo<'info>,
    pub delegate: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct InitializeAccount<'info> {
    pub account: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CloseAccount<'info> {
    pub account: AccountInfo<'info>,
    pub destination: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct FreezeAccount<'info> {
    pub account: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct ThawAccount<'info> {
    pub account: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct InitializeMint<'info> {
    pub mint: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SetAuthority<'info> {
    pub current_authority: AccountInfo<'info>,
    pub account_or_mint: AccountInfo<'info>,
}

#[derive(Clone)]
pub struct TokenAccount(spl_token::state::Account);

impl TokenAccount {
    pub const LEN: usize = spl_token::state::Account::LEN;
}

impl anchor_lang::AccountDeserialize for TokenAccount {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        TokenAccount::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        spl_token::state::Account::unpack(buf).map(TokenAccount)
    }
}

impl anchor_lang::AccountSerialize for TokenAccount {
    fn try_serialize<W: Write>(&self, _writer: &mut W) -> Result<(), ProgramError> {
        // no-op
        Ok(())
    }
}

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

#[derive(Clone)]
pub struct Mint(spl_token::state::Mint);

impl Mint {
    pub const LEN: usize = spl_token::state::Mint::LEN;
}

impl anchor_lang::AccountDeserialize for Mint {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        Mint::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        spl_token::state::Mint::unpack(buf).map(Mint)
    }
}

impl anchor_lang::AccountSerialize for Mint {
    fn try_serialize<W: Write>(&self, _writer: &mut W) -> Result<(), ProgramError> {
        // no-op
        Ok(())
    }
}

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

impl anchor_lang::AccountDeserialize for Token {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        Token::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(_buf: &mut &[u8]) -> Result<Self, ProgramError> {
        Ok(Token)
    }
}

impl anchor_lang::Id for Token {
    fn id() -> Pubkey {
        ID
    }
}

// Field parsers to save compute. All account validation is assumed to be done
// outside of these methods.
pub mod accessor {
    use super::*;

    pub fn amount(account: &AccountInfo) -> Result<u64, ProgramError> {
        let bytes = account.try_borrow_data()?;
        let mut amount_bytes = [0u8; 8];
        amount_bytes.copy_from_slice(&bytes[64..72]);
        Ok(u64::from_le_bytes(amount_bytes))
    }

    pub fn mint(account: &AccountInfo) -> Result<Pubkey, ProgramError> {
        let bytes = account.try_borrow_data()?;
        let mut mint_bytes = [0u8; 32];
        mint_bytes.copy_from_slice(&bytes[..32]);
        Ok(Pubkey::new_from_array(mint_bytes))
    }

    pub fn authority(account: &AccountInfo) -> Result<Pubkey, ProgramError> {
        let bytes = account.try_borrow_data()?;
        let mut owner_bytes = [0u8; 32];
        owner_bytes.copy_from_slice(&bytes[32..64]);
        Ok(Pubkey::new_from_array(owner_bytes))
    }
}
