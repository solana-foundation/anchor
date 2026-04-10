//! Well-known program marker types for use with `Program<T>`.

use pinocchio::address::Address;
use crate::Id;

pub struct System;
impl Id for System {
    fn id() -> Address { Address::from_str_const("11111111111111111111111111111111") }
}

pub struct Token;
impl Id for Token {
    fn id() -> Address { Address::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA") }
}

pub struct Token2022;
impl Id for Token2022 {
    fn id() -> Address { Address::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb") }
}

pub struct AssociatedToken;
impl Id for AssociatedToken {
    fn id() -> Address { Address::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL") }
}

pub struct Memo;
impl Id for Memo {
    fn id() -> Address { Address::from_str_const("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr") }
}
