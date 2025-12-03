//! Defines the [`AbsolutePath`] trait and implementations for the various commands
//! and sub-commands.

use std::path::PathBuf;

/// Used to get the absolute form of all paths within this type
pub(crate) trait AbsolutePath: Sized {
    /// Convert all [`PathBuf`]s within this to absolute.
    fn absolute(self) -> Self;
}

impl AbsolutePath for PathBuf {
    fn absolute(self) -> Self {
        std::path::absolute(&self)
            .unwrap_or_else(|e| panic!("failed to get absolute path for `{}`: {e}", self.display()))
    }
}

impl<T: AbsolutePath> AbsolutePath for Option<T> {
    fn absolute(self) -> Self {
        self.map(T::absolute)
    }
}

impl<T: AbsolutePath> AbsolutePath for Vec<T> {
    fn absolute(self) -> Self {
        self.into_iter().map(T::absolute).collect()
    }
}

// For types with no paths, implement a no-op `absolute`
macro_rules! impl_nop {
    ($($ty:path),* $(,)?) => {
        $(
            impl AbsolutePath for $ty {
                fn absolute(self) -> Self {
                    self
                }
            }
        )*
    };
}

impl_nop! {
    // Primitives
    bool,
    u8,
    u32,
    u64,
    usize,
    f64,

    // std types
    String,

    // First-party types
    anchor_client::Cluster,

    // Third-party types
    clap_complete::Shell,
    solana_commitment_config::CommitmentLevel,
    solana_pubkey::Pubkey,
}
