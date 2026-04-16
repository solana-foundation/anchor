//! Well-known program marker types for use with `Program<T>`.
//!
//! Program IDs are const-evaluated via `from_str_const` stored in `const` bindings
//! so base58 decoding happens at compile time, not at runtime.
//!
//! Each marker also exposes `IDL_ADDRESS: &'static str` (gated behind the
//! `idl-build` feature), which `Program<T>` forwards through
//! `IdlAccountType::__IDL_ADDRESS` at IDL emission time. Replaces the old
//! hardcoded 5-entry address table that used to live in
//! `derive/src/parse.rs::extract_program_address`.

use {crate::Id, pinocchio::address::Address};

pub struct System;
impl Id for System {
    fn id() -> Address {
        const ADDR: Address = Address::from_str_const("11111111111111111111111111111111");
        ADDR
    }
    #[cfg(feature = "idl-build")]
    const IDL_ADDRESS: &'static str = "11111111111111111111111111111111";
}

pub struct Token;
impl Id for Token {
    fn id() -> Address {
        const ADDR: Address =
            Address::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        ADDR
    }
    #[cfg(feature = "idl-build")]
    const IDL_ADDRESS: &'static str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
}

pub struct Token2022;
impl Id for Token2022 {
    fn id() -> Address {
        const ADDR: Address =
            Address::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
        ADDR
    }
    #[cfg(feature = "idl-build")]
    const IDL_ADDRESS: &'static str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
}

pub struct AssociatedToken;
impl Id for AssociatedToken {
    fn id() -> Address {
        const ADDR: Address =
            Address::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
        ADDR
    }
    #[cfg(feature = "idl-build")]
    const IDL_ADDRESS: &'static str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
}

pub struct Memo;
impl Id for Memo {
    fn id() -> Address {
        const ADDR: Address =
            Address::from_str_const("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
        ADDR
    }
    #[cfg(feature = "idl-build")]
    const IDL_ADDRESS: &'static str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
}
