//! SPL Token mint type with `AccountValidate` impl for use with `Account<T>`.
//!
//! Layout mirrors `pinocchio-token` — all fields are alignment-1 to support
//! zerocopy mapping from the account data buffer.

use {
    crate::token::create_token_account,
    anchor_lang_v2::{
        accounts::{Account, AccountInitialize, AccountValidate},
        programs::{Token, Token2022},
        Constrain, Id,
    },
    bytemuck::{Pod, Zeroable},
    pinocchio::account::AccountView,
    solana_address::Address,
    solana_program_error::ProgramError,
};

/// SPL Token mint data, zerocopy-mapped (82 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Mint {
    /// COption tag for mint_authority.
    mint_authority_flag: [u8; 4],
    /// Optional authority used to mint new tokens.
    pub mint_authority: Address,
    /// Total supply of tokens.
    supply: [u8; 8],
    /// Number of decimals.
    pub decimals: u8,
    /// Is initialized.
    is_initialized: u8,
    /// COption tag for freeze_authority.
    freeze_authority_flag: [u8; 4],
    /// Optional authority to freeze token accounts.
    pub freeze_authority: Address,
}

// SAFETY: Mint is repr(C) with all alignment-1 fields, no padding.
unsafe impl Pod for Mint {}
unsafe impl Zeroable for Mint {}

impl AccountValidate for Mint {
    fn validate(view: &AccountView, data: &[u8], _program_id: &Address) -> Result<(), ProgramError> {
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

/// Init params for `#[account(init, mint::decimals = 6, mint::authority = ..., ...)]`.
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

        create_token_account(payer, account, core::mem::size_of::<Self>(), signer_seeds)?;

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
    pub fn supply(&self) -> u64 {
        u64::from_le_bytes(self.supply)
    }

    pub fn has_mint_authority(&self) -> bool {
        self.mint_authority_flag[0] == 1
    }

    pub fn mint_authority(&self) -> Option<&Address> {
        if self.has_mint_authority() { Some(&self.mint_authority) } else { None }
    }

    pub fn is_initialized(&self) -> bool {
        self.is_initialized == 1
    }

    pub fn has_freeze_authority(&self) -> bool {
        self.freeze_authority_flag[0] == 1
    }

    pub fn freeze_authority(&self) -> Option<&Address> {
        if self.has_freeze_authority() { Some(&self.freeze_authority) } else { None }
    }
}

// ---------------------------------------------------------------------------
// Constraint markers for `#[account(mint::*)]`
// ---------------------------------------------------------------------------

pub struct AuthorityConstraint;
pub struct FreezeAuthorityConstraint;
pub struct DecimalsConstraint;
pub struct TokenProgramConstraint;

impl Constrain<AuthorityConstraint> for Account<Mint> {
    fn constrain(&self, expected: &Address) -> Result<(), ProgramError> {
        if !self.has_mint_authority() || self.mint_authority != *expected {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(())
        }
    }
}

impl Constrain<FreezeAuthorityConstraint> for Account<Mint> {
    fn constrain(&self, expected: &Address) -> Result<(), ProgramError> {
        if !self.has_freeze_authority() || self.freeze_authority != *expected {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(())
        }
    }
}

/// `mint::Decimals = 6` — non-address constraint, compares u8.
impl Constrain<DecimalsConstraint, u8> for Account<Mint> {
    fn constrain(&self, expected: &u8) -> Result<(), ProgramError> {
        if self.decimals != *expected {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(())
        }
    }
}

/// `mint::TokenProgram = token_program` — check mint is owned by given program.
impl Constrain<TokenProgramConstraint> for Account<Mint> {
    fn constrain(&self, expected: &Address) -> Result<(), ProgramError> {
        if !AsRef::<AccountView>::as_ref(self).owned_by(expected) {
            Err(ProgramError::IllegalOwner)
        } else {
            Ok(())
        }
    }
}
