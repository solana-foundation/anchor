//! IDL emission trait — type metadata for the IDL, parallel to
//! [`crate::AnchorAccount`] for runtime loading.
//!
//! Dispatches on the wrapper type: default returns `None` (elides
//! sysvar/signer/program/unchecked from IDL types). Data-bearing wrappers
//! (`Box<T>`, `Account<T>`, `BorshAccount<T>`, `Slab<H, T>`, `Nested<T>`)
//! delegate to the inner type. User `#[account]`/`#[event]`/`#[derive(IdlType)]`
//! structs get auto-generated impls with `__IDL_TYPE = Some(<JSON>)` and
//! recursive `__register_idl_deps`.
//!
//! Only present under `idl-build` cfg; compiles out for on-chain builds.

extern crate alloc;

/// Contributes (or elides) a user-defined type to the generated IDL.
///
/// `__IDL_TYPE = None` → framework wrapper, nothing added to `types[]`.
/// `__IDL_TYPE = Some(json)` → contributes a types entry.
/// `__IDL_IS_SIGNER` / `__IDL_ADDRESS` surface per-wrapper metadata in
/// `instructions[i].accounts[j]` JSON (`Signer`, `Program<T>`, `Sysvar<T>`).
///
/// `__register_idl_deps` handles transitive type registration: user structs
/// push their type def and recurse into fields. Primitives/collections
/// default to no-op; wrappers delegate to the inner type.
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
    u8,
    u16,
    u32,
    u64,
    u128,
    i8,
    i16,
    i32,
    i64,
    i128,
    f32,
    f64,
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

// Borrowed slice `&[T]` — surfaces on `#[derive(IdlType)]` structs that
// carry borrowed slice fields (e.g. `MixedArgs<'a> { values: &'a [u64] }`),
// which wincode supports as a zero-copy ix arg.
impl<T: IdlAccountType> IdlAccountType for [T] {
    fn __register_idl_deps(types: &mut alloc::vec::Vec<&'static str>) {
        T::__register_idl_deps(types);
    }
}

// `&T` — forward to `T` so a field typed as `&'a Inner` pulls `Inner`'s
// type def into the IDL's `types[]`.
impl<T: IdlAccountType + ?Sized> IdlAccountType for &T {
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

// `PodVec<T, MAX>` — the zero-copy bounded-capacity analog of `Vec<T>`.
// Forward `__register_idl_deps` so a `#[account]` zero-copy type holding
// a `PodVec<Inner, 16>` still pulls `Inner` into the IDL's `types[]`.
impl<T, const MAX: usize> IdlAccountType for crate::pod::PodVec<T, MAX>
where
    T: bytemuck::Pod + IdlAccountType,
{
    fn __register_idl_deps(types: &mut alloc::vec::Vec<&'static str>) {
        T::__register_idl_deps(types);
    }
}

// ---------------------------------------------------------------------------
// Runtime seed-const JSON emission
// ---------------------------------------------------------------------------
//
// Used by the `Accounts` derive when a `seeds = [...]` entry is an arbitrary
// const-evaluatable expression that the macro can't byte-fold itself
// (e.g. `MY_PREFIX`, `crate::seeds::TAG`, `<Marker as Id>::id()`). The
// derive emits a runtime call to this helper from inside `__idl_accounts()`,
// turning anything that implements `AsRef<[u8]>` into the IDL's
// `{"kind":"const","value":[...]}` shape. Centralized so the byte-formatting
// is testable and lives in one place.
#[doc(hidden)]
pub fn __idl_const_seed_json(value: impl AsRef<[u8]>) -> alloc::string::String {
    let bytes = value.as_ref();
    let mut s = alloc::string::String::from(r#"{"kind":"const","value":["#);
    let mut first = true;
    for b in bytes {
        if !first {
            s.push(',');
        }
        first = false;
        s.push_str(&alloc::format!("{b}"));
    }
    s.push_str("]}");
    s
}

#[cfg(test)]
mod const_seed_json_tests {
    use {super::*, pinocchio::address::Address};

    #[test]
    fn empty_bytes_render_empty_value_array() {
        assert_eq!(
            __idl_const_seed_json([0u8; 0]),
            r#"{"kind":"const","value":[]}"#
        );
    }

    #[test]
    fn array_of_bytes_renders_decimal_values() {
        // Boundary values exercise the format path: 0, mid, max u8.
        assert_eq!(
            __idl_const_seed_json([0u8, 1, 99, 255]),
            r#"{"kind":"const","value":[0,1,99,255]}"#
        );
    }

    #[test]
    fn string_literal_renders_as_utf8_bytes() {
        // Surfaces the `&str` path used by `seeds = [MY_STR]` patterns.
        assert_eq!(
            __idl_const_seed_json("ab"),
            r#"{"kind":"const","value":[97,98]}"#
        );
    }

    #[test]
    fn byte_slice_renders_in_order() {
        let s: &[u8] = &[10u8, 20, 30];
        assert_eq!(
            __idl_const_seed_json(s),
            r#"{"kind":"const","value":[10,20,30]}"#
        );
    }

    #[test]
    fn pinocchio_address_lands_as_32_decimal_bytes() {
        // The const-fallback case: `<Marker>::id()` returns Address; the
        // helper has to accept it via the AsRef<[u8]> blanket. Pick a
        // base58 with a recognizable byte pattern.
        let addr = Address::from_str_const("11111111111111111111111111111111");
        let json = __idl_const_seed_json(addr);
        assert!(
            json.starts_with(r#"{"kind":"const","value":["#),
            "missing prefix: {json}"
        );
        assert!(json.ends_with("]}"), "missing suffix: {json}");
        // System program id is the all-zero pubkey — 32 zeros.
        assert_eq!(
            json,
            r#"{"kind":"const","value":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}"#
        );
    }
}
