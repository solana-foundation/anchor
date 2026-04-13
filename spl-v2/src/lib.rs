//! Anchor v2 SPL account types and constraint markers.
//!
//! Separate crate (like v1's `anchor-spl`) that provides zero-copy `TokenAccount`
//! and `Mint` types for use with `Account<T>`, plus namespaced constraint markers
//! for `token::mint`, `token::authority`, `mint::decimals`, etc.

#![no_std]

pub mod associated_token;
pub mod mint;
pub mod token;

pub use associated_token::get_associated_token_address;
pub use token::{TokenAccount, TokenAccountInitParams};
pub use mint::{Mint, MintInitParams};
