//! Legacy IDL instruction support has been removed in favor of Program Metadata.
//!
//! This module now only provides the IDL build feature for generating IDLs
//! without injecting instructions into programs.

#[cfg(feature = "idl-build")]
pub use anchor_lang_idl::{build::IdlBuild, *};
