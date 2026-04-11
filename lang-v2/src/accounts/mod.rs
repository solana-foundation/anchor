mod unchecked_account;
mod signer;
mod system_account;
mod program;
mod boxed;
mod option;
mod borsh_account;
mod account;
mod sysvar;

pub use unchecked_account::UncheckedAccount;
pub use signer::Signer;
pub use system_account::SystemAccount;
pub use program::Program;
pub use option::Optional;
pub use borsh_account::BorshAccount;
pub use account::{Account, AccountValidate, AccountInitialize};
pub use sysvar::{Sysvar, SysvarId};

/// Generates `Deref<Target=AccountView>` + `AsRef<AccountView>` + `AsRef<Address>`
/// for a view wrapper that stores its `AccountView` in a field named `view`.
///
/// Covers only the mechanical trait delegation — validation logic and any
/// extra inherent methods (e.g. `address()`) still live in the concrete type's
/// own file. Not used by `Account<T>` / `BorshAccount<T>` (non-`AccountView`
/// `Deref::Target`) or `Program<T>` (generic bounds).
macro_rules! view_wrapper_traits {
    ($Type:ty) => {
        impl core::ops::Deref for $Type {
            type Target = pinocchio::account::AccountView;
            #[inline(always)]
            fn deref(&self) -> &pinocchio::account::AccountView { &self.view }
        }
        impl AsRef<pinocchio::account::AccountView> for $Type {
            #[inline(always)]
            fn as_ref(&self) -> &pinocchio::account::AccountView { &self.view }
        }
        impl AsRef<pinocchio::address::Address> for $Type {
            #[inline(always)]
            fn as_ref(&self) -> &pinocchio::address::Address { self.view.address() }
        }
    };
}
pub(crate) use view_wrapper_traits;
