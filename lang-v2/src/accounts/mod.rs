mod unchecked_account;
mod signer;
mod system_account;
mod program;
mod boxed;
mod option;
mod borsh_account;
mod slab;
mod slab_hooks;
mod sysvar;

pub use unchecked_account::UncheckedAccount;
pub use signer::Signer;
pub use system_account::SystemAccount;
pub use program::Program;
pub use option::Optional;
pub use borsh_account::BorshAccount;
pub use slab::{Slab, HeaderOnly};
pub use slab_hooks::{SlabInit, SlabValidate};
pub use crate::AccountInitialize;
pub use sysvar::{Sysvar, SysvarId};

/// Anchor account with a typed header and no trailing items.
///
/// This is the common case — a one-struct-per-account layout where the
/// account's data bytes are `[disc][T]`. It's a thin type alias over
/// [`Slab<T, HeaderOnly>`], which means `Account<T>` shares all of `Slab`'s
/// validation, borrow-tracking, init, and close machinery. The layout is
/// byte-identical to the pre-Slab `Account<T>` (the `HeaderOnly` marker is
/// a ZST that doesn't implement `Pod`, so the tail-only impl block never
/// matches and the length field is never emitted), so existing on-chain
/// accounts stay readable and no migration is required.
///
/// Tail-only methods (`len`, `push`, `as_slice`, etc.) are compile errors
/// on `Account<T>` — they live in an `impl<H, T> Slab<H, T> where T: Pod`
/// block that `HeaderOnly` doesn't satisfy. The error is the standard
/// "method not found" from the compiler.
///
/// For accounts with a length-prefixed tail, use [`Slab<H, T>`] directly:
/// ```ignore
/// #[derive(Accounts)]
/// pub struct Grow<'info> {
///     #[account(mut)]
///     pub ledger: Slab<Ledger, Entry>,  // tail of `Entry` items
/// }
/// ```
pub type Account<T> = slab::Slab<T, HeaderOnly>;

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
