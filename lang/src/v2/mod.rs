//! Anchor v2: Trait-based account system with Pinocchio runtime.
//!
//! This module provides the [`AnchorAccount`] trait, a unified interface for all
//! account types in Anchor. Instead of per-type codegen in the `#[derive(Accounts)]`
//! macro, each account type implements this trait, and the macro simply calls
//! `T::load()` / `T::load_mut()` uniformly.

pub mod accounts;
mod context;
mod cpi;
pub mod programs;
mod traits;

pub use pinocchio::account::AccountView;
pub use pinocchio::address::Address;
pub use context::Context;
pub use cpi::{create_account, create_account_signed, find_program_address};
pub use traits::*;

/// Re-export msg for use in generated code.
pub use solana_msg::msg;

// Re-export the v2 derive macros
pub use anchor_derive_accounts_v2::Accounts;
pub use anchor_derive_accounts_v2::AnchorData;
pub use anchor_derive_accounts_v2::account;
pub use anchor_derive_accounts_v2::program;

// Re-export v1 event system (works as-is with v2)
pub use anchor_attribute_event::{emit, event};

// Re-export serialization derives needed by #[event]
pub use crate::{AnchorSerialize, AnchorDeserialize};
// Re-export v1 traits needed by #[event] / emit! macro expansion
pub use crate::Discriminator as EventDiscriminator;
pub use crate::Event;
