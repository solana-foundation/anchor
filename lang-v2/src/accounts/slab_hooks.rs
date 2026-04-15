//! Hooks that `Slab<H, T>` requires from its header type `H`. Factored out
//! of `slab.rs` so the Slab machinery itself stays focused on the wrapper
//! logic (load/validate/deref/close).
//!
//! - [`SlabValidate`] — bytes-level validation (owner, discriminator, size).
//!   Every `#[account]` type gets a default via the `Owner + Discriminator`
//!   blanket; SPL `Mint` / `TokenAccount` override directly.
//! - [`SlabInit`] — bytes-level init (create + disc write by default, SPL
//!   CPI for `Mint`/`TokenAccount`). Invoked by `Slab<H, _>`'s
//!   `AccountInitialize` forward impl.

use {
    crate::{Discriminator, Owner},
    pinocchio::{account::AccountView, address::Address},
    solana_program_error::ProgramError,
};

/// Validation hook Slab runs on its header type's bytes before mapping.
///
/// Types marked with `#[account]` get the blanket impl below. External
/// types (SPL `Mint` / `TokenAccount`) implement this directly with custom
/// validation (exact-length checks, no discriminator).
pub trait SlabValidate {
    /// Byte offset where `Self`'s data starts in the account buffer.
    /// - Anchor native types (`#[account]`): 8 (discriminator length)
    /// - External types (SPL `Mint` / `TokenAccount`): 0
    const DATA_OFFSET: usize;

    fn validate(view: &AccountView, data: &[u8], program_id: &Address) -> Result<(), ProgramError>;

    #[inline(always)]
    fn data_offset() -> usize {
        Self::DATA_OFFSET
    }
}

impl<T: Owner + Discriminator> SlabValidate for T {
    const DATA_OFFSET: usize = 8;

    #[inline(always)]
    fn validate(view: &AccountView, data: &[u8], program_id: &Address) -> Result<(), ProgramError> {
        if !view.owned_by(&T::owner(program_id)) {
            return Err(super::slab::cold_owner_error(view));
        }
        let disc = T::DISCRIMINATOR;
        let min_len = disc.len() + core::mem::size_of::<T>();
        if data.len() < min_len {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if &data[..disc.len()] != disc {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}

/// Bytes-writing hook Slab invokes during `init`. The default blanket
/// (`T: Owner + Discriminator`) does `create_account` + write disc; SPL
/// `Mint` / `TokenAccount` override with their own token-program CPIs.
///
/// Not needed for self-contained wrappers — those impl
/// [`AccountInitialize`](crate::AccountInitialize) directly.
pub trait SlabInit {
    type Params<'a>: Default;

    fn create_and_initialize<'a>(
        payer: &AccountView,
        account: &AccountView,
        space: usize,
        program_id: &Address,
        params: &Self::Params<'a>,
        signer_seeds: Option<&[&[u8]]>,
    ) -> Result<(), ProgramError>;
}

impl<T: Owner + Discriminator> SlabInit for T {
    type Params<'a> = ();

    #[inline(always)]
    fn create_and_initialize<'a>(
        payer: &AccountView,
        account: &AccountView,
        space: usize,
        program_id: &Address,
        _params: &(),
        signer_seeds: Option<&[&[u8]]>,
    ) -> Result<(), ProgramError> {
        let disc: &[u8; 8] = T::DISCRIMINATOR
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?;
        match signer_seeds {
            Some(seeds) => crate::create_account_signed(payer, account, space, program_id, seeds)?,
            None => crate::create_account(payer, account, space, program_id)?,
        }
        let mut account_view = *account;
        let data = unsafe { account_view.borrow_unchecked_mut() };
        match data.first_chunk_mut::<8>() {
            Some(dst) => *dst = *disc,
            None => return Err(ProgramError::AccountDataTooSmall),
        }
        Ok(())
    }
}
