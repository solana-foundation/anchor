//! Account types that can be used in the account validation struct.

pub mod account;
pub mod account_loader;
pub mod account_view;
pub mod boxed;
pub mod interface;
pub mod interface_account;
pub mod migration;
pub mod option;
pub mod sysvar;

/// Executable program account as `Account<Program<T>>` (marker: [`crate::accounts::account::Program`]).
pub mod program {
    pub type Program<T = crate::accounts::account::AnyProgram> =
        crate::accounts::account::Account<crate::accounts::account::Program<T>>;
}

#[cfg(feature = "lazy-account")]
pub mod lazy_account;
