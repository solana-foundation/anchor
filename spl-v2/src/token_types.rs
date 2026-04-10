//! SPL Token account types with `AccountValidate` impls for use with `Account<T>`.
//!
//! This crate is external to anchor-lang-v2 — proves BYOC constraint system works.
//! Users write `use anchor_spl_v2::token;` and `#[account(token::Mint = mint)]`.

use {
    bytemuck::{Pod, Zeroable},
    pinocchio::account::AccountView,
    solana_address::Address,
    solana_program_error::ProgramError,
    anchor_lang_v2::{
        accounts::{Account, AccountValidate, AccountInitialize},
        constraints::Constrain,
        Id,
    },
};

// Program IDs — const-evaluated
struct Token;
impl Id for Token {
    fn id() -> Address { const ADDR: Address = Address::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"); ADDR }
}

struct Token2022;
impl Id for Token2022 {
    fn id() -> Address { const ADDR: Address = Address::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"); ADDR }
}

// ---------------------------------------------------------------------------
// TokenAccount (165 bytes)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TokenAccount {
    pub mint: Address,
    pub authority: Address,
    amount: [u8; 8],
    delegate_flag: [u8; 4],
    pub delegate: Address,
    pub state: u8,
    is_native_flag: [u8; 4],
    native_amount: [u8; 8],
    delegated_amount: [u8; 8],
    close_authority_flag: [u8; 4],
    pub close_authority: Address,
}

unsafe impl Pod for TokenAccount {}
unsafe impl Zeroable for TokenAccount {}

impl AccountValidate for TokenAccount {
    fn validate(view: &AccountView, data: &[u8]) -> Result<(), ProgramError> {
        if !view.owned_by(&Token::id()) && !view.owned_by(&Token2022::id()) {
            return Err(ProgramError::IllegalOwner);
        }
        if data.len() != core::mem::size_of::<Self>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
    fn data_offset() -> usize { 0 }
}

#[derive(Default)]
pub struct TokenAccountInitParams<'a> {
    pub mint: Option<&'a AccountView>,
    pub authority: Option<&'a AccountView>,
}

impl AccountInitialize for TokenAccount {
    type Params<'a> = TokenAccountInitParams<'a>;

    fn create_and_initialize<'a>(
        payer: &AccountView,
        account: &AccountView,
        _space: usize,
        _program_id: &Address,
        params: &Self::Params<'a>,
        signer_seeds: Option<&[&[u8]]>,
    ) -> Result<(), ProgramError> {
        let mint = params.mint.ok_or(ProgramError::InvalidArgument)?;
        let authority = params.authority.ok_or(ProgramError::InvalidArgument)?;
        let token_program_id = Token::id();
        match signer_seeds {
            Some(seeds) => anchor_lang_v2::create_account_signed(
                payer, account, core::mem::size_of::<Self>(), &token_program_id, seeds,
            )?,
            None => anchor_lang_v2::create_account(
                payer, account, core::mem::size_of::<Self>(), &token_program_id,
            )?,
        }
        pinocchio_token::instructions::InitializeAccount3 {
            account,
            mint,
            owner: authority.address(),
        }
        .invoke()
    }
}

impl TokenAccount {
    pub fn amount(&self) -> u64 { u64::from_le_bytes(self.amount) }
    pub fn delegated_amount(&self) -> u64 { u64::from_le_bytes(self.delegated_amount) }
    pub fn has_delegate(&self) -> bool { self.delegate_flag[0] == 1 }
    pub fn delegate(&self) -> Option<&Address> { if self.has_delegate() { Some(&self.delegate) } else { None } }
    pub fn is_native(&self) -> bool { self.is_native_flag[0] == 1 }
    pub fn native_amount(&self) -> Option<u64> { if self.is_native() { Some(u64::from_le_bytes(self.native_amount)) } else { None } }
    pub fn has_close_authority(&self) -> bool { self.close_authority_flag[0] == 1 }
    pub fn close_authority(&self) -> Option<&Address> { if self.has_close_authority() { Some(&self.close_authority) } else { None } }
    pub fn is_initialized(&self) -> bool { self.state != 0 }
    pub fn is_frozen(&self) -> bool { self.state == 2 }
}

// ---------------------------------------------------------------------------
// Mint (82 bytes)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Mint {
    mint_authority_flag: [u8; 4],
    pub mint_authority: Address,
    supply: [u8; 8],
    pub decimals: u8,
    is_initialized: u8,
    freeze_authority_flag: [u8; 4],
    pub freeze_authority: Address,
}

unsafe impl Pod for Mint {}
unsafe impl Zeroable for Mint {}

impl AccountValidate for Mint {
    fn validate(view: &AccountView, data: &[u8]) -> Result<(), ProgramError> {
        if !view.owned_by(&Token::id()) && !view.owned_by(&Token2022::id()) {
            return Err(ProgramError::IllegalOwner);
        }
        if data.len() != core::mem::size_of::<Self>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
    fn data_offset() -> usize { 0 }
}

#[derive(Default)]
pub struct MintInitParams<'a> {
    pub decimals: Option<u8>,
    pub authority: Option<&'a AccountView>,
    pub freeze_authority: Option<&'a AccountView>,
}

impl AccountInitialize for Mint {
    type Params<'a> = MintInitParams<'a>;

    fn create_and_initialize<'a>(
        payer: &AccountView,
        account: &AccountView,
        _space: usize,
        _program_id: &Address,
        params: &Self::Params<'a>,
        signer_seeds: Option<&[&[u8]]>,
    ) -> Result<(), ProgramError> {
        let decimals = params.decimals.ok_or(ProgramError::InvalidArgument)?;
        let authority = params.authority.ok_or(ProgramError::InvalidArgument)?;
        let token_program_id = Token::id();
        match signer_seeds {
            Some(seeds) => anchor_lang_v2::create_account_signed(
                payer, account, core::mem::size_of::<Self>(), &token_program_id, seeds,
            )?,
            None => anchor_lang_v2::create_account(
                payer, account, core::mem::size_of::<Self>(), &token_program_id,
            )?,
        }
        pinocchio_token::instructions::InitializeMint2 {
            mint: account,
            decimals,
            mint_authority: authority.address(),
            freeze_authority: params.freeze_authority.map(|v| v.address()),
        }
        .invoke()
    }
}

impl Mint {
    pub fn supply(&self) -> u64 { u64::from_le_bytes(self.supply) }
    pub fn has_mint_authority(&self) -> bool { self.mint_authority_flag[0] == 1 }
    pub fn mint_authority(&self) -> Option<&Address> { if self.has_mint_authority() { Some(&self.mint_authority) } else { None } }
    pub fn is_initialized(&self) -> bool { self.is_initialized == 1 }
    pub fn has_freeze_authority(&self) -> bool { self.freeze_authority_flag[0] == 1 }
    pub fn freeze_authority(&self) -> Option<&Address> { if self.has_freeze_authority() { Some(&self.freeze_authority) } else { None } }
}

// ---------------------------------------------------------------------------
// Constraint marker types — BYOC: these live in spl-v2, not in anchor core.
// Users import: `use anchor_spl_v2::token;`
// Then write:   `#[account(token::Mint = mint_account)]`
// ---------------------------------------------------------------------------

pub mod token {
    pub struct Mint;
    pub struct Authority;
    pub struct TokenProgram;
}

pub mod mint {
    pub struct Authority;
    pub struct FreezeAuthority;
}

// ---------------------------------------------------------------------------
// Constrain impls
// ---------------------------------------------------------------------------

impl Constrain<token::Mint> for Account<TokenAccount> {
    fn constrain(&self, expected: &Address) -> Result<(), ProgramError> {
        if self.mint != *expected { Err(ProgramError::InvalidAccountData) } else { Ok(()) }
    }
}

impl Constrain<token::Authority> for Account<TokenAccount> {
    fn constrain(&self, expected: &Address) -> Result<(), ProgramError> {
        if self.authority != *expected { Err(ProgramError::InvalidAccountData) } else { Ok(()) }
    }
}

impl Constrain<mint::Authority> for Account<Mint> {
    fn constrain(&self, expected: &Address) -> Result<(), ProgramError> {
        if !self.has_mint_authority() || self.mint_authority != *expected { Err(ProgramError::InvalidAccountData) } else { Ok(()) }
    }
}

impl Constrain<mint::FreezeAuthority> for Account<Mint> {
    fn constrain(&self, expected: &Address) -> Result<(), ProgramError> {
        if !self.has_freeze_authority() || self.freeze_authority != *expected { Err(ProgramError::InvalidAccountData) } else { Ok(()) }
    }
}
