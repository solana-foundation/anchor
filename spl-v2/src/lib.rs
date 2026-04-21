//! Anchor v2 SPL account types and constraint markers.
//!
//! Separate crate (like v1's `anchor-spl`) that provides zero-copy `TokenAccount`
//! and `Mint` types for use with `Account<T>`, plus namespaced constraint markers
//! for `token::mint`, `token::authority`, `mint::decimals`, etc.
//!
//! For programs that need to accept both Token and Token-2022 accounts, use the
//! `token_interface` module which provides `InterfaceAccount<T>`.

#![no_std]

pub mod associated_token;
pub mod extensions;
pub mod mint;
pub mod token;
pub mod token_interface;

/// Re-export `pinocchio-token-2022` for Token-2022 extension CPI instructions.
pub use pinocchio_token_2022 as token_2022_cpi;

pub use {
    associated_token::get_associated_token_address,
    mint::{Mint, MintInitParams},
    token::{TokenAccount, TokenAccountInitParams},
    token_interface::{InterfaceAccount, Interface},
};
