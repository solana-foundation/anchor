use satellite_lang::Bumps;
use arch_program::{program_error::ProgramError, utxo::UtxoMeta};
use satellite_bitcoin::utxo_info::UtxoInfo;

use crate::context::BtcContext;

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
    // Fallback: minimal stub with just the metadata.  Value/rune information
    // will be default-initialised; predicates depending on those will fail.
    let mut info = UtxoInfo::default();
    info.meta = meta.clone();
    Ok(info)
}

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
/// [`UtxoInfo`]: satellite_bitcoin::utxo_info::UtxoInfo
pub trait TryFromUtxos<'info, A: Bumps>: Sized {
    /// Parse and validate a slice of [`UtxoMeta`].
    ///
    /// * `ctx`  – mutable reference to the [`BtcContext`] carrying the Bitcoin builder and
    ///            validated `Accounts` struct.
    /// * `utxos` – slice of UTXO metadata to parse.
    fn try_utxos(
        ctx: &mut BtcContext<'_, '_, '_, '_, 'info, A>,
        utxos: &[arch_program::utxo::UtxoMeta],
    ) -> Result<Self, ProgramError>;
}
