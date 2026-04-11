//! SPL Token account type with `AccountValidate` impl for use with `Account<T>`.
//!
//! Layout mirrors `pinocchio-token` — all fields are alignment-1 to support
//! zerocopy mapping from the account data buffer.

use {
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

/// Create a Token-program-owned account, handling PDA signing if needed.
pub(crate) fn create_token_account(
    payer: &AccountView,
    account: &AccountView,
    space: usize,
    signer_seeds: Option<&[&[u8]]>,
) -> Result<(), ProgramError> {
    let token_program_id = Token::id();
    match signer_seeds {
        Some(seeds) => anchor_lang_v2::create_account_signed(payer, account, space, &token_program_id, seeds),
        None => anchor_lang_v2::create_account(payer, account, space, &token_program_id),
    }
}

/// SPL Token account data, zerocopy-mapped (165 bytes).
///
/// All fields are private — use the accessor methods to read data. Token
/// account state is modified only by the SPL Token program via CPI (Transfer,
/// MintTo, etc.); user programs cannot mutate these fields directly anyway
/// because the account is owned by the SPL Token program.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TokenAccount {
    mint: Address,
    owner: Address,
    amount: [u8; 8],
    delegate_flag: [u8; 4],
    delegate: Address,
    state: u8,
    is_native_flag: [u8; 4],
    native_amount: [u8; 8],
    delegated_amount: [u8; 8],
    close_authority_flag: [u8; 4],
    close_authority: Address,
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
    /// The mint associated with this token account.
    pub fn mint(&self) -> &Address {
        &self.mint
    }

    /// The owner of this token account.
    pub fn owner(&self) -> &Address {
        &self.owner
    }

    /// The token balance.
    pub fn amount(&self) -> u64 {
        u64::from_le_bytes(self.amount)
    }

    /// The amount currently delegated.
    pub fn delegated_amount(&self) -> u64 {
        u64::from_le_bytes(self.delegated_amount)
    }

    /// Whether a delegate is currently approved.
    pub fn has_delegate(&self) -> bool {
        self.delegate_flag[0] == 1
    }

    /// The approved delegate, if any.
    pub fn delegate(&self) -> Option<&Address> {
        if self.has_delegate() { Some(&self.delegate) } else { None }
    }

    /// Account state (0 = Uninitialized, 1 = Initialized, 2 = Frozen).
    pub fn state(&self) -> u8 {
        self.state
    }

    /// Whether this is a wrapped SOL account.
    pub fn is_native(&self) -> bool {
        self.is_native_flag[0] == 1
    }

    /// The rent-exempt reserve for native SOL accounts, if this is a native
    /// token account.
    pub fn native_amount(&self) -> Option<u64> {
        if self.is_native() { Some(u64::from_le_bytes(self.native_amount)) } else { None }
    }

    /// Whether a close authority is set.
    pub fn has_close_authority(&self) -> bool {
        self.close_authority_flag[0] == 1
    }

    /// The close authority, if any.
    pub fn close_authority(&self) -> Option<&Address> {
        if self.has_close_authority() { Some(&self.close_authority) } else { None }
    }

    /// Whether the account has been initialized (state != 0).
    pub fn is_initialized(&self) -> bool {
        self.state != 0
    }

    /// Whether the account is frozen (state == 2).
    pub fn is_frozen(&self) -> bool {
        self.state == 2
    }
}

// ---------------------------------------------------------------------------
// Constraint markers for `#[account(token::*)]`
// ---------------------------------------------------------------------------

pub struct MintConstraint;
pub struct AuthorityConstraint;
pub struct TokenProgramConstraint;

impl Constrain<MintConstraint> for Account<TokenAccount> {
    fn constrain(&self, expected: &Address) -> Result<(), ProgramError> {
        if *self.mint() != *expected {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(())
        }
    }
}

impl Constrain<AuthorityConstraint> for Account<TokenAccount> {
    fn constrain(&self, expected: &Address) -> Result<(), ProgramError> {
        if *self.owner() != *expected {
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
