//! IDL emission trait — parallel to [`crate::AnchorAccount`], but scoped to
//! type metadata rather than runtime loading.
//!
//! `#[derive(Accounts)]` dispatches on the *wrapper* (`Program<T>`, `Sysvar<T>`,
//! `Account<T>`, `BorshAccount<T>`, …) to decide whether a field contributes a
//! user-defined type to the IDL's `types`/`accounts` sections. The default
//! impl returns `None`, so sysvar / signer / program / unchecked fields are
//! cleanly elided from IDL type collection — without requiring us to reach
//! into their `::Data` associated type (which for `Sysvar<Clock>` is a foreign
//! struct with no `__IDL_TYPE` to speak of).
//!
//! Data-bearing wrappers (`Box<T>`, `Account<T>`, `BorshAccount<T>`,
//! `Slab<H, T>`, `Nested<T>`) delegate to the inner type's `__IDL_TYPE`.
//!
//! User `#[account]` structs get this impl generated automatically, with
//! `__IDL_TYPE = Some(<JSON>)`.
//!
//! This trait is only present under the `idl-build` cfg — it compiles out
//! entirely for on-chain builds.

/// Contributes (or elides) a user-defined type to the generated IDL.
///
/// Returning `None` for `__IDL_TYPE` means "this field's Rust type is a
/// framework wrapper around a foreign / view-only type; don't add anything
/// to the IDL's `types` section for it." Returning `Some(json)` means
/// "this field's Rust type (or its inner `#[account]` type, after
/// transparent wrapper delegation) should contribute `json` as a types
/// entry."
///
/// `__IDL_IS_SIGNER` and `__IDL_ADDRESS` capture per-wrapper metadata that
/// surfaces in the emitted `instructions[i].accounts[j]` JSON: `Signer`
/// sets `__IDL_IS_SIGNER = true`; `Program<T: Id>` and `Sysvar<T>` forward
/// well-known addresses through their respective `IDL_ADDRESS` const paths.
/// Replaces the old string-match + hardcoded address table in
/// `derive/src/parse.rs`.
pub trait IdlAccountType {
    const __IDL_TYPE: Option<&'static str> = None;
    const __IDL_IS_SIGNER: bool = false;
    const __IDL_ADDRESS: Option<&'static str> = None;
}
