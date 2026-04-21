//! Miri baseline for spl-v2 Pod types.
//!
//! Verifies that `TokenAccount` and `Mint` can be cast to/from their
//! byte representations without UB under Tree Borrows. The types are
//! declared `unsafe impl Pod + Zeroable` based on a manual layout check
//! (`repr(C)`, alignment-1 fields, no padding); Miri verifies that the
//! Pod contract actually holds under aliasing rules.
//!
//! Run: `MIRIFLAGS="-Zmiri-tree-borrows" cargo +nightly miri test -p anchor-spl-v2 --test miri_spl_pod`

use anchor_spl_v2::{Mint, TokenAccount};
use solana_program_pack::Pack;

// --- Layout cross-check against canonical `spl-token-interface` packed sizes.
//
// `size_of::<MyType>()` == `Pack::LEN` ties the anchor-spl-v2 Pod layout to
// the protocol-level packed size exposed by the interface crate. If SPL Token
// ever changes the wire layout, this test fails until we catch up.

#[test]
fn mint_size_matches_spl_token_interface() {
    assert_eq!(
        core::mem::size_of::<Mint>(),
        spl_token_interface::state::Mint::LEN,
    );
}

#[test]
fn token_account_size_matches_spl_token_interface() {
    assert_eq!(
        core::mem::size_of::<TokenAccount>(),
        spl_token_interface::state::Account::LEN,
    );
}

#[test]
fn token_account_zeroed_is_valid() {
    // `Zeroable` produces a valid TokenAccount. If the Pod impl is
    // unsound (hidden padding), Miri would flag the cast below.
    let acct: TokenAccount = bytemuck::Zeroable::zeroed();
    let bytes: &[u8] = bytemuck::bytes_of(&acct);
    assert_eq!(bytes.len(), 165);
    assert!(bytes.iter().all(|&b| b == 0));
}

#[test]
fn mint_zeroed_is_valid() {
    let m: Mint = bytemuck::Zeroable::zeroed();
    let bytes: &[u8] = bytemuck::bytes_of(&m);
    assert_eq!(bytes.len(), 82);
    assert!(bytes.iter().all(|&b| b == 0));
}

#[test]
fn token_account_byte_roundtrip() {
    // Cast arbitrary 165 bytes into TokenAccount, then back — must be
    // byte-identical.
    let src: Vec<u8> = (0u8..165).collect();
    let acct: &TokenAccount = bytemuck::from_bytes(&src);
    let bytes: &[u8] = bytemuck::bytes_of(acct);
    assert_eq!(bytes, src.as_slice());
}

#[test]
fn mint_byte_roundtrip() {
    let src: Vec<u8> = (0u8..82).collect();
    let m: &Mint = bytemuck::from_bytes(&src);
    let bytes: &[u8] = bytemuck::bytes_of(m);
    assert_eq!(bytes, src.as_slice());
}

#[test]
fn wrong_size_rejects_cast() {
    // bytemuck::from_bytes panics on size mismatch — important as a
    // safety boundary for TokenAccount (165) vs. Mint (82).
    let src = vec![0u8; 100];
    let result = std::panic::catch_unwind(|| {
        let _: &TokenAccount = bytemuck::from_bytes(&src);
    });
    assert!(result.is_err(), "bytemuck::from_bytes should reject mismatched size");
}
