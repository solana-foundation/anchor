//! Regression tests for `AccountLoader::{load, load_mut, load_init}` panicking
//! on accounts that pass the discriminator length check but whose data is
//! truncated before the end of the zero-copy body.
//!
//! Before the fix, the three accessors sliced
//!     data[disc.len()..disc.len() + size_of::<T>()]
//! without verifying the upper bound, which caused an index-out-of-bounds
//! panic (surfaced to clients as `Program failed to complete`) instead of a
//! structured `AnchorError`. `load_init` additionally panicked on the
//! discriminator slice itself when the account was shorter than the
//! discriminator.
//!
//! Tracking: solana-foundation/anchor#4509

use anchor_lang::error::ErrorCode;
use anchor_lang::prelude::*;
use std::mem;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[account(zero_copy)]
#[derive(Default, Debug)]
pub struct Foo {
    pub a: u64,
    pub b: u64,
    pub c: u64,
}

/// Build an `AccountInfo` whose data is exactly `len` bytes long. If
/// `with_disc` is true the discriminator prefix is written; otherwise the
/// buffer is zeroed.
fn make_owned_account_bytes(len: usize, with_disc: bool) -> Vec<u8> {
    let mut data = vec![0u8; len];
    if with_disc {
        let disc = Foo::DISCRIMINATOR;
        let n = std::cmp::min(disc.len(), len);
        data[..n].copy_from_slice(&disc[..n]);
    }
    data
}

#[test]
fn load_returns_error_when_data_shorter_than_zero_copy_body() {
    // Discriminator-sized but truncated body — used to panic.
    let mut data = make_owned_account_bytes(Foo::DISCRIMINATOR.len(), true);
    let mut lamports: u64 = 1;
    let owner: Pubkey = crate::ID;
    let key: Pubkey = Pubkey::new_unique();

    let acc_info: AccountInfo<'_> =
        AccountInfo::new(&key, false, false, &mut lamports, &mut data, &owner, false);

    let loader: AccountLoader<'_, Foo> = AccountLoader::try_from(&acc_info).unwrap();
    let err = loader.load().unwrap_err();
    assert_eq!(
        err,
        anchor_lang::error::Error::from(ErrorCode::AccountDidNotDeserialize)
    );
}

#[test]
fn load_mut_returns_error_when_data_shorter_than_zero_copy_body() {
    // One byte short of the full zero-copy body — used to panic.
    let needed = Foo::DISCRIMINATOR.len() + mem::size_of::<Foo>();
    let mut data = make_owned_account_bytes(needed - 1, true);
    let mut lamports: u64 = 1;
    let owner: Pubkey = crate::ID;
    let key: Pubkey = Pubkey::new_unique();

    let acc_info: AccountInfo<'_> =
        AccountInfo::new(&key, false, true, &mut lamports, &mut data, &owner, false);

    let loader: AccountLoader<'_, Foo> = AccountLoader::try_from(&acc_info).unwrap();
    let err = loader.load_mut().unwrap_err();
    assert_eq!(
        err,
        anchor_lang::error::Error::from(ErrorCode::AccountDidNotDeserialize)
    );
}

#[test]
fn load_init_returns_error_when_data_shorter_than_zero_copy_body() {
    // Discriminator-region is zero (as init expects) but body is missing.
    let mut data = make_owned_account_bytes(Foo::DISCRIMINATOR.len(), false);
    let mut lamports: u64 = 1;
    let owner: Pubkey = crate::ID;
    let key: Pubkey = Pubkey::new_unique();

    let acc_info: AccountInfo<'_> =
        AccountInfo::new(&key, false, true, &mut lamports, &mut data, &owner, false);

    let loader: AccountLoader<'_, Foo> =
        AccountLoader::try_from_unchecked(&crate::ID, &acc_info).unwrap();
    let err = loader.load_init().unwrap_err();
    assert_eq!(
        err,
        anchor_lang::error::Error::from(ErrorCode::AccountDidNotDeserialize)
    );
}

#[test]
fn load_init_returns_error_when_data_shorter_than_discriminator() {
    // Below `disc.len()` — `load_init` previously sliced without bounds-checking
    // the discriminator prefix and panicked.
    let mut data = vec![0u8; Foo::DISCRIMINATOR.len() - 1];
    let mut lamports: u64 = 1;
    let owner: Pubkey = crate::ID;
    let key: Pubkey = Pubkey::new_unique();

    let acc_info: AccountInfo<'_> =
        AccountInfo::new(&key, false, true, &mut lamports, &mut data, &owner, false);

    let loader: AccountLoader<'_, Foo> =
        AccountLoader::try_from_unchecked(&crate::ID, &acc_info).unwrap();
    let err = loader.load_init().unwrap_err();
    assert_eq!(
        err,
        anchor_lang::error::Error::from(ErrorCode::AccountDiscriminatorNotFound)
    );
}

#[test]
fn load_succeeds_on_exactly_sized_account() {
    // Sanity: the happy path is unaffected by the new bound check.
    let needed = Foo::DISCRIMINATOR.len() + mem::size_of::<Foo>();
    let mut data = make_owned_account_bytes(needed, true);
    let mut lamports: u64 = 1;
    let owner: Pubkey = crate::ID;
    let key: Pubkey = Pubkey::new_unique();

    let acc_info: AccountInfo<'_> =
        AccountInfo::new(&key, false, true, &mut lamports, &mut data, &owner, false);

    let loader: AccountLoader<'_, Foo> = AccountLoader::try_from(&acc_info).unwrap();
    {
        let foo = loader.load().unwrap();
        assert_eq!(foo.a, 0);
    }
    {
        let mut foo = loader.load_mut().unwrap();
        foo.a = 7;
    }
    assert_eq!(loader.load().unwrap().a, 7);
}
