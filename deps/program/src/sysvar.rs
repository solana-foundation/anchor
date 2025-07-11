use crate::{account::AccountInfo, program_error::ProgramError};

/// Minimal replacement for Solana's `sysvar::Sysvar` trait.
///
/// The full Solana trait contains many helper functions that are unnecessary
/// for Arch right now.  Anchor only relies on the two methods below
/// (`from_account_info` and `get`), so this stripped-down version
/// intentionally keeps the surface area small.  Additional methods can be
/// added later if / when the need arises.
pub trait Sysvar: Sized {
    /// Deserialize the sysvar from the given `AccountInfo` instance.
    fn from_account_info(account_info: &AccountInfo) -> Result<Self, ProgramError>;

    /// Fetch the sysvar directly from the runtime via a dedicated syscall.
    fn get() -> Result<Self, ProgramError>;
}
