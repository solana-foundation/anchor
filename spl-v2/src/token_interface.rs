//! Interface account types that accept both Token and Token-2022 programs.
//!
//! Provides `InterfaceAccount<T>` — a type alias for accounts that can be
//! owned by either `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` (Token) or
//! `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb` (Token-2022).
//!
//! # Usage
//!
//! ```ignore
//! use anchor_spl_v2::token_interface::{InterfaceAccount, TokenAccount, Mint};
//!
//! #[derive(Accounts)]
//! pub struct MyAccounts {
//!     #[account(token::mint = mint, token::authority = owner)]
//!     pub token_account: InterfaceAccount<TokenAccount>,
//!     pub mint: InterfaceAccount<Mint>,
//! }
//! ```

use {
    anchor_lang_v2::{
        accounts::{Account, SlabInit, SlabSchema},
        programs::{Token, Token2022},
        Constrain, Id,
    },
    bytemuck::{Pod, Zeroable},
    core::ops::Deref,
    pinocchio::account::AccountView,
    solana_address::Address,
    solana_program_error::ProgramError,
};

// ---------------------------------------------------------------------------
// Interface<T> — transparent wrapper that changes validation to accept both
// Token and Token-2022 program ownership.
// ---------------------------------------------------------------------------

/// Transparent wrapper around an SPL type `T` that relaxes ownership
/// validation to accept both the Token and Token-2022 programs.
///
/// Users should not reference this type directly — use `InterfaceAccount<T>`.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Interface<T>(T);

// SAFETY: Interface<T> is #[repr(transparent)] over T.
// If T is Pod+Zeroable, so is Interface<T>.
unsafe impl<T: Pod> Pod for Interface<T> {}
unsafe impl<T: Zeroable> Zeroable for Interface<T> {}

impl<T> Deref for Interface<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        &self.0
    }
}

/// Account type alias that accepts both Token and Token-2022 programs.
///
/// Equivalent to v1's `InterfaceAccount<'info, T>` but without a lifetime.
pub type InterfaceAccount<T> = Account<Interface<T>>;

// ---------------------------------------------------------------------------
// SlabSchema — Interface<TokenAccount>
// ---------------------------------------------------------------------------

impl SlabSchema for Interface<crate::TokenAccount> {
    const DATA_OFFSET: usize = 0;

    #[inline(always)]
    fn validate(
        view: &AccountView,
        data: &[u8],
        _program_id: &Address,
    ) -> Result<(), ProgramError> {
        if !view.owned_by(&Token::id()) && !view.owned_by(&Token2022::id()) {
            return Err(ProgramError::IllegalOwner);
        }
        // Token-2022 accounts may be larger than 165 bytes (extensions follow
        // the base state). The first 165 bytes are always the base layout.
        if data.len() < core::mem::size_of::<crate::TokenAccount>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SlabSchema — Interface<Mint>
// ---------------------------------------------------------------------------

impl SlabSchema for Interface<crate::Mint> {
    const DATA_OFFSET: usize = 0;

    #[inline(always)]
    fn validate(
        view: &AccountView,
        data: &[u8],
        _program_id: &Address,
    ) -> Result<(), ProgramError> {
        if !view.owned_by(&Token::id()) && !view.owned_by(&Token2022::id()) {
            return Err(ProgramError::IllegalOwner);
        }
        if data.len() < core::mem::size_of::<crate::Mint>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Space
// ---------------------------------------------------------------------------

impl anchor_lang_v2::Space for Interface<crate::TokenAccount> {
    const INIT_SPACE: usize = core::mem::size_of::<crate::TokenAccount>();
}

impl anchor_lang_v2::Space for Interface<crate::Mint> {
    const INIT_SPACE: usize = core::mem::size_of::<crate::Mint>();
}

// ---------------------------------------------------------------------------
// IDL — keep interface types out of the user's types[] array
// ---------------------------------------------------------------------------

#[cfg(feature = "idl-build")]
impl anchor_lang_v2::IdlAccountType for Interface<crate::TokenAccount> {}

#[cfg(feature = "idl-build")]
impl anchor_lang_v2::IdlAccountType for Interface<crate::Mint> {}

// ---------------------------------------------------------------------------
// SlabInit — Interface<TokenAccount>
// ---------------------------------------------------------------------------

/// Init params for `InterfaceAccount<TokenAccount>`. Requires `token_program`
/// to know which program to create the account through.
#[derive(Default)]
pub struct InterfaceTokenAccountInitParams<'a> {
    pub mint: Option<&'a AccountView>,
    pub authority: Option<&'a AccountView>,
    pub token_program: Option<&'a AccountView>,
}

impl SlabInit for Interface<crate::TokenAccount> {
    type Params<'a> = InterfaceTokenAccountInitParams<'a>;

    #[cold]
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
        let token_program = params
            .token_program
            .ok_or(ProgramError::InvalidArgument)?;
        let program_id = token_program.address();

        let space = core::mem::size_of::<crate::TokenAccount>();
        match signer_seeds {
            Some(seeds) => {
                anchor_lang_v2::create_account_signed(payer, account, space, program_id, seeds)
            }
            None => anchor_lang_v2::create_account(payer, account, space, program_id),
        }?;

        pinocchio_token::instructions::InitializeAccount3 {
            account,
            mint,
            owner: authority.address(),
        }
        .invoke()
    }
}

// ---------------------------------------------------------------------------
// SlabInit — Interface<Mint>
// ---------------------------------------------------------------------------

/// Init params for `InterfaceAccount<Mint>`.
#[derive(Default)]
pub struct InterfaceMintInitParams<'a> {
    pub decimals: Option<u8>,
    pub authority: Option<&'a AccountView>,
    pub freeze_authority: Option<&'a AccountView>,
    pub token_program: Option<&'a AccountView>,
}

impl SlabInit for Interface<crate::Mint> {
    type Params<'a> = InterfaceMintInitParams<'a>;

    #[cold]
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
        let token_program = params
            .token_program
            .ok_or(ProgramError::InvalidArgument)?;
        let program_id = token_program.address();

        let space = core::mem::size_of::<crate::Mint>();
        match signer_seeds {
            Some(seeds) => {
                anchor_lang_v2::create_account_signed(payer, account, space, program_id, seeds)
            }
            None => anchor_lang_v2::create_account(payer, account, space, program_id),
        }?;

        pinocchio_token::instructions::InitializeMint2 {
            mint: account,
            decimals,
            mint_authority: authority.address(),
            freeze_authority: params.freeze_authority.map(|v| v.address()),
        }
        .invoke()
    }
}

// ---------------------------------------------------------------------------
// Constraint impls — token::* on InterfaceAccount<TokenAccount>
// ---------------------------------------------------------------------------

macro_rules! impl_token_account_constraints {
    ($target:ty) => {
        impl Constrain<crate::token::MintConstraint> for $target {
            #[inline(always)]
            fn constrain(&mut self, expected: &Address) -> Result<(), ProgramError> {
                if !anchor_lang_v2::address_eq(self.mint(), expected) {
                    Err(ProgramError::InvalidAccountData)
                } else {
                    Ok(())
                }
            }
        }

        impl Constrain<crate::token::AuthorityConstraint> for $target {
            #[inline(always)]
            fn constrain(&mut self, expected: &Address) -> Result<(), ProgramError> {
                if !anchor_lang_v2::address_eq(self.owner(), expected) {
                    Err(ProgramError::InvalidAccountData)
                } else {
                    Ok(())
                }
            }
        }

        impl Constrain<crate::token::TokenProgramConstraint> for $target {
            #[inline(always)]
            fn constrain(&mut self, expected: &Address) -> Result<(), ProgramError> {
                if !AsRef::<AccountView>::as_ref(self).owned_by(expected) {
                    Err(ProgramError::IllegalOwner)
                } else {
                    Ok(())
                }
            }
        }
    };
}

impl_token_account_constraints!(InterfaceAccount<crate::TokenAccount>);

// ---------------------------------------------------------------------------
// Constraint impls — mint::* on InterfaceAccount<Mint>
// ---------------------------------------------------------------------------

macro_rules! impl_mint_constraints {
    ($target:ty) => {
        impl Constrain<crate::mint::AuthorityConstraint> for $target {
            #[inline(always)]
            fn constrain(&mut self, expected: &Address) -> Result<(), ProgramError> {
                match self.mint_authority() {
                    Some(addr) if addr == expected => Ok(()),
                    _ => Err(ProgramError::InvalidAccountData),
                }
            }
        }

        impl Constrain<crate::mint::FreezeAuthorityConstraint> for $target {
            #[inline(always)]
            fn constrain(&mut self, expected: &Address) -> Result<(), ProgramError> {
                match self.freeze_authority() {
                    Some(addr) if addr == expected => Ok(()),
                    _ => Err(ProgramError::InvalidAccountData),
                }
            }
        }

        impl Constrain<crate::mint::DecimalsConstraint, u8> for $target {
            #[inline(always)]
            fn constrain(&mut self, expected: &u8) -> Result<(), ProgramError> {
                if self.decimals() != *expected {
                    Err(ProgramError::InvalidAccountData)
                } else {
                    Ok(())
                }
            }
        }

        impl Constrain<crate::mint::TokenProgramConstraint> for $target {
            #[inline(always)]
            fn constrain(&mut self, expected: &Address) -> Result<(), ProgramError> {
                if !AsRef::<AccountView>::as_ref(self).owned_by(expected) {
                    Err(ProgramError::IllegalOwner)
                } else {
                    Ok(())
                }
            }
        }
    };
}

impl_mint_constraints!(InterfaceAccount<crate::Mint>);
