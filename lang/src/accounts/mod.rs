//! Account types that can be used in the account validation struct.

pub mod account;
pub mod account_info;
pub mod account_loader;
pub mod boxed;
pub mod interface;
pub mod interface_account;
pub mod option;
pub mod program;
pub mod signer;
pub mod system_account;
pub mod sysvar;
pub mod unchecked_account;

// Collection type that groups a variable number of account/account-loader items.
pub mod shards;

#[cfg(feature = "lazy-account")]
pub mod lazy_account;
