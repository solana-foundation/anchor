//! Anchor IDL.

pub mod types;

#[cfg(feature = "build")]
pub mod build;

#[cfg(feature = "build")]
pub use serde_json;
