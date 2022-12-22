//! Account types that can be used in the account validation struct.

pub mod account;
pub mod account_info;
pub mod account_loader;
pub mod boxed;
#[doc(hidden)]
#[allow(deprecated)]
pub mod cpi_account;
#[doc(hidden)]
#[allow(deprecated)]
pub mod loader;
pub mod option;
pub mod program;
#[doc(hidden)]
#[allow(deprecated)]
pub mod program_account;
pub mod signer;
pub mod system_account;
pub mod sysvar;
pub mod unchecked_account;
