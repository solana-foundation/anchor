pub mod prelude {
    pub use crate::TryFromUtxos;
}

use anchor_lang::Bumps;
use arch_program::{program_error::ProgramError, utxo::UtxoMeta};
use saturn_bitcoin_transactions::utxo_info::UtxoInfo;

// Bring in the host-side test registry when compiling off-chain.
#[cfg(not(target_os = "solana"))]
mod test_registry;

#[cfg(not(target_os = "solana"))]
pub use test_registry::register_test_utxo_info;

// -----------------------------------------------------------------------------
// meta_to_info implementation
// -----------------------------------------------------------------------------
/// Convert a [`UtxoMeta`] into a full [`UtxoInfo`] while compiling for the
/// Solana BPF target we rely on the real on-chain syscall; when building for
/// the host we fall back to a lightweight stub that avoids the syscall.
#[cfg(target_os = "solana")]
pub fn meta_to_info(meta: &UtxoMeta) -> Result<UtxoInfo, ProgramError> {
    UtxoInfo::try_from(meta)
}

#[cfg(not(target_os = "solana"))]
pub fn meta_to_info(meta: &UtxoMeta) -> Result<UtxoInfo, ProgramError> {
    // If the test registered a rich UtxoInfo for this meta, use it.
    if let Some(info) = test_registry::lookup(meta) {
        return Ok(info);
    }

    // Fallback: minimal stub with just the metadata.  Value/rune information
    // will be default-initialised; predicates depending on those will fail.
    let mut info = UtxoInfo::default();
    info.meta = meta.clone();
    Ok(info)
}

pub mod error;
pub use error::ErrorCode;
/// Core trait for parsing and validating UTXO information.
///
/// This trait converts a slice of [`UtxoInfo`] into a strongly-typed
/// structure that matches your parsing requirements. This is the main trait
/// implemented by the [`UtxoParser`] derive macro.
///
/// Unlike [`TryFromUtxoMetas`], this trait operates directly on the rich
/// [`UtxoInfo`] type, which provides access to full transaction details,
/// rune information, and other metadata needed for comprehensive validation.
///
/// ## Implementation
///
/// Typically you won't implement this trait manually. Instead, use the
/// [`UtxoParser`] derive macro which generates the implementation automatically
/// based on your struct definition and `#[utxo(...)]` attributes.
///
/// [`UtxoInfo`]: saturn_bitcoin_transactions::utxo_info::UtxoInfo
pub trait TryFromUtxos<'utxos, 'a, 'b, 'c, 'info, T: Bumps>: Sized {
    /// Parse and validate a slice of [`UtxoMeta`].
    ///
    /// * `ctx`   – reference to the Anchor [`BtcContext`] holding the validated
    ///             account struct generated via `#[derive(Accounts)]`.
    /// * `utxos` – slice of UTXO metadata to parse.
    fn try_utxos(
        ctx: &mut anchor_lang::context::BtcContext<'a, 'b, 'c, 'info, T>,
        utxos: &'utxos [arch_program::utxo::UtxoMeta],
    ) -> Result<Self, ProgramError>;
}

/// Re-export the derive macro so downstream crates need only one dependency.
///
/// This allows users to import both the trait and derive macro from the same crate:
///
/// ```rust
/// use saturn_utxo_parser::{UtxoParser, TryFromUtxos};
///
/// #[derive(UtxoParser)]
/// struct MyParser {
///     // ... fields with #[utxo(...)] attributes
/// }
/// ```
pub use saturn_utxo_parser_derive::UtxoParser;