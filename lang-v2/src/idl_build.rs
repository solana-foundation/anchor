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
//! User `#[account]` / `#[event]` / `#[derive(IdlType)]` structs get this impl
//! generated automatically, with `__IDL_TYPE = Some(<JSON>)` plus an
//! `__register_idl_deps` body that walks each field type so nested user
//! structs land in the IDL's `types[]` array too.
//!
//! This trait is only present under the `idl-build` cfg — it compiles out
//! entirely for on-chain builds.

extern crate alloc;

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
///
/// `__register_idl_deps` powers transitive type registration. A struct that
/// appears as a field inside another `#[account]` / `#[event]` / `#[derive(
/// IdlType)]` body gets its `__IDL_TYPE` pushed into the accumulator **and**
/// recurses into its own fields. Primitives / collections default to the
/// no-op impl; framework wrappers delegate to the inner type.
pub trait IdlAccountType {
    const __IDL_TYPE: Option<&'static str> = None;
    const __IDL_IS_SIGNER: bool = false;
    const __IDL_ADDRESS: Option<&'static str> = None;

    /// Register this type's `__IDL_TYPE` (if any) and recursively register
    /// any user-defined types its fields reference. Default: no-op.
    ///
    /// Implementers that carry their own type def push it here; delegating
    /// wrappers (`Box<T>`, `BorshAccount<T>`, `Slab<H, T>`, `Nested<T>`)
    /// forward to their inner type; collection impls (`Vec<T>`, `Option<T>`,
    /// `[T; N]`) forward to the element type. Primitive impls (bool, u*,
    /// i*, f*, String, Address, etc.) use the default no-op — they never
    /// appear in `types[]`.
    fn __register_idl_deps(_types: &mut alloc::vec::Vec<&'static str>) {}
}

// ---------------------------------------------------------------------------
// Primitive + collection blanket impls
// ---------------------------------------------------------------------------
//
// All default to the no-op `__register_idl_deps` so a struct field of these
// types doesn't contribute anything to `types[]`. The collection impls
// forward to their element type so a `Vec<Inner>` field still pulls `Inner`
// into the registry.

macro_rules! impl_idl_account_type_noop {
    ($($t:ty),* $(,)?) => {
        $(
            impl IdlAccountType for $t {}
        )*
    };
}

impl_idl_account_type_noop!(
    bool,
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
    f32, f64,
    alloc::string::String,
    pinocchio::address::Address,
);

// Pod integer wrappers — treated the same as their native counterparts for
// IDL purposes (they map to `"u64"`, `"i32"`, etc. via `rust_type_to_idl`'s
// string-based dispatch). The blanket impl here keeps the trait resolvable
// when users reference them from nested structs.
impl_idl_account_type_noop!(
    crate::pod::PodBool,
    crate::pod::PodU16,
    crate::pod::PodU32,
    crate::pod::PodU64,
    crate::pod::PodU128,
    crate::pod::PodI16,
    crate::pod::PodI32,
    crate::pod::PodI64,
    crate::pod::PodI128,
);

impl<T: IdlAccountType> IdlAccountType for alloc::vec::Vec<T> {
    fn __register_idl_deps(types: &mut alloc::vec::Vec<&'static str>) {
        T::__register_idl_deps(types);
    }
}

impl<T: IdlAccountType> IdlAccountType for Option<T> {
    fn __register_idl_deps(types: &mut alloc::vec::Vec<&'static str>) {
        T::__register_idl_deps(types);
    }
}

impl<T: IdlAccountType, const N: usize> IdlAccountType for [T; N] {
    fn __register_idl_deps(types: &mut alloc::vec::Vec<&'static str>) {
        T::__register_idl_deps(types);
    }
}
