//! SPL Token account types with `AccountValidate` impls for use with `Account<T>`.
//!
//! Layout mirrors `pinocchio-token` — all fields are alignment-1 to support
//! zerocopy mapping from the account data buffer.

use {
    bytemuck::{Pod, Zeroable},
    pinocchio::account::AccountView,
    solana_address::Address,
    solana_program_error::ProgramError,
    super::account::AccountValidate,
    crate::programs::{Token, Token2022},
    crate::Id,
};

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
    fn validate(view: &AccountView, data: &[u8]) -> Result<(), ProgramError> {
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
