//! SPL Token account types with `AccountValidate` impls for use with `Account<T>`.
//!
//! Layout mirrors `pinocchio-token` — all fields are alignment-1 to support
//! zerocopy mapping from the account data buffer.

use {
    bytemuck::{Pod, Zeroable},
    pinocchio::account::AccountView,
    solana_address::Address,
    solana_program_error::ProgramError,
    super::{account::{AccountValidate, AccountInitialize}, Account},
    crate::Constrain,
    crate::programs::{Token, Token2022},
    crate::Id,
};

/// Create a Token-program-owned account, handling PDA signing if needed.
fn create_token_account(
    payer: &AccountView,
    account: &AccountView,
    space: usize,
    signer_seeds: Option<&[&[u8]]>,
) -> Result<(), ProgramError> {
    let token_program_id = Token::id();
    match signer_seeds {
        Some(seeds) => crate::create_account_signed(payer, account, space, &token_program_id, seeds),
        None => crate::create_account(payer, account, space, &token_program_id),
    }
}

// ---------------------------------------------------------------------------
// TokenAccount (165 bytes)
// ---------------------------------------------------------------------------

/// SPL Token account data, zerocopy-mapped.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TokenAccount {
    /// The mint associated with this account.
    pub mint: Address,
    /// The owner of this account.
    pub authority: Address,
    /// The amount of tokens this account holds.
    amount: [u8; 8],
    /// COption tag for delegate.
    delegate_flag: [u8; 4],
    /// Optional delegate.
    pub delegate: Address,
    /// Account state (0=Uninitialized, 1=Initialized, 2=Frozen).
    pub state: u8,
    /// COption tag for is_native.
    is_native_flag: [u8; 4],
    /// Rent-exempt reserve for native tokens.
    native_amount: [u8; 8],
    /// The amount delegated.
    delegated_amount: [u8; 8],
    /// COption tag for close_authority.
    close_authority_flag: [u8; 4],
    /// Optional authority to close the account.
    pub close_authority: Address,
}

// SAFETY: TokenAccount is repr(C) with all alignment-1 fields, no padding.
unsafe impl Pod for TokenAccount {}
unsafe impl Zeroable for TokenAccount {}

impl AccountValidate for TokenAccount {
    fn validate(view: &AccountView, data: &[u8], _program_id: &Address) -> Result<(), ProgramError> {
        // Token accounts can be owned by Token or Token2022.
        if !view.owned_by(&Token::id()) && !view.owned_by(&Token2022::id()) {
            return Err(ProgramError::IllegalOwner);
        }
        // Exact size distinguishes TokenAccount (165) from Mint (82).
        if data.len() != core::mem::size_of::<Self>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
    fn data_offset() -> usize { 0 }
}

/// Init params for `#[account(init, token::mint = ..., token::authority = ...)]`.
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

        create_token_account(payer, account, core::mem::size_of::<Self>(), signer_seeds)?;

        pinocchio_token::instructions::InitializeAccount3 {
            account,
            mint,
            owner: authority.address(),
        }
        .invoke()
    }
}

impl TokenAccount {
    pub fn amount(&self) -> u64 {
        u64::from_le_bytes(self.amount)
    }

    pub fn delegated_amount(&self) -> u64 {
        u64::from_le_bytes(self.delegated_amount)
    }

    pub fn has_delegate(&self) -> bool {
        self.delegate_flag[0] == 1
    }

    pub fn delegate(&self) -> Option<&Address> {
        if self.has_delegate() { Some(&self.delegate) } else { None }
    }

    pub fn is_native(&self) -> bool {
        self.is_native_flag[0] == 1
    }

    pub fn native_amount(&self) -> Option<u64> {
        if self.is_native() { Some(u64::from_le_bytes(self.native_amount)) } else { None }
    }

    pub fn has_close_authority(&self) -> bool {
        self.close_authority_flag[0] == 1
    }

    pub fn close_authority(&self) -> Option<&Address> {
        if self.has_close_authority() { Some(&self.close_authority) } else { None }
    }

    pub fn is_initialized(&self) -> bool {
        self.state != 0
    }

    pub fn is_frozen(&self) -> bool {
        self.state == 2
    }
}

// ---------------------------------------------------------------------------
// Mint (82 bytes)
// ---------------------------------------------------------------------------

/// SPL Token mint data, zerocopy-mapped.
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
// Constraint markers + impls
// ---------------------------------------------------------------------------

/// Constraint markers for `token::*` constraints on `Account<TokenAccount>`.
pub struct MintConstraint;
pub struct AuthorityConstraint;
pub struct TokenProgramConstraint;

/// Constraint markers for `mint::*` constraints on `Account<Mint>`.
pub mod mint {
    pub struct AuthorityConstraint;
    pub struct FreezeAuthorityConstraint;
    pub struct DecimalsConstraint;
    pub struct TokenProgramConstraint;
}

impl Constrain<MintConstraint> for Account<TokenAccount> {
    fn constrain(&self, expected: &Address) -> Result<(), ProgramError> {
        if self.mint != *expected {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(())
        }
    }
}

impl Constrain<AuthorityConstraint> for Account<TokenAccount> {
    fn constrain(&self, expected: &Address) -> Result<(), ProgramError> {
        if self.authority != *expected {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Constrain impls — Account<Mint>
// ---------------------------------------------------------------------------

impl Constrain<mint::AuthorityConstraint> for Account<Mint> {
    fn constrain(&self, expected: &Address) -> Result<(), ProgramError> {
        if !self.has_mint_authority() || self.mint_authority != *expected {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(())
        }
    }
}

impl Constrain<mint::FreezeAuthorityConstraint> for Account<Mint> {
    fn constrain(&self, expected: &Address) -> Result<(), ProgramError> {
        if !self.has_freeze_authority() || self.freeze_authority != *expected {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(())
        }
    }
}

/// `mint::Decimals = 6` — non-address constraint, compares u8.
impl Constrain<mint::DecimalsConstraint, u8> for Account<Mint> {
    fn constrain(&self, expected: &u8) -> Result<(), ProgramError> {
        if self.decimals != *expected {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(())
        }
    }
}

/// `token::TokenProgram = token_program` — check account is owned by given program.
impl Constrain<TokenProgramConstraint> for Account<TokenAccount> {
    fn constrain(&self, expected: &Address) -> Result<(), ProgramError> {
        if !AsRef::<AccountView>::as_ref(self).owned_by(expected) {
            Err(ProgramError::IllegalOwner)
        } else {
            Ok(())
        }
    }
}

/// `mint::TokenProgram = token_program` — check mint is owned by given program.
impl Constrain<mint::TokenProgramConstraint> for Account<Mint> {
    fn constrain(&self, expected: &Address) -> Result<(), ProgramError> {
        if !AsRef::<AccountView>::as_ref(self).owned_by(expected) {
            Err(ProgramError::IllegalOwner)
        } else {
            Ok(())
        }
    }
}
