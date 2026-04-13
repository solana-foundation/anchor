use {
    core::ops::Deref,
    pinocchio::{
        account::AccountView,
        address::Address,
    },
    solana_program_error::ProgramError,
    crate::AnchorAccount,
};

/// Wrapper for optional accounts.
///
/// Users write `Option<Account<T>>` in their source. The `#[derive(Accounts)]`
/// macro translates it to `Optional<Account<T>>` in the generated struct.
///
/// Sentinel convention: if the client passes the program's own ID as the
/// account address, it's interpreted as "not provided" (None).
///
/// Access pattern: `if let Some(account) = ctx.accounts.maybe.as_ref() { ... }`
pub struct Optional<T: AnchorAccount>(pub Option<T>);

impl<T: AnchorAccount> AnchorAccount for Optional<T> {
    /// Derefs to `Option<T>`, giving access to `as_ref()`, `is_some()`, etc.
    type Data = Option<T>;

    fn load(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        // Sentinel: client passes the program's own address to mean `None`.
        // Use `address_eq` for the chunked compare — see lib.rs rationale.
        if crate::address_eq(view.address(), program_id) {
            Ok(Self(None))
        } else {
            Ok(Self(Some(T::load(view, program_id)?)))
        }
    }

    fn load_mut(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        if crate::address_eq(view.address(), program_id) {
            Ok(Self(None))
        } else {
            Ok(Self(Some(T::load_mut(view, program_id)?)))
        }
    }

    fn account(&self) -> &AccountView {
        self.0
            .as_ref()
            .expect("cannot access account of None optional")
            .account()
    }

    fn exit(&mut self) -> pinocchio::ProgramResult {
        if let Some(ref mut inner) = self.0 {
            inner.exit()
        } else {
            Ok(())
        }
    }

    fn close(&mut self, destination: AccountView) -> pinocchio::ProgramResult {
        if let Some(ref mut inner) = self.0 {
            inner.close(destination)
        } else {
            Ok(())
        }
    }
}

impl<T: AnchorAccount> Deref for Optional<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Option<T> {
        &self.0
    }
}

impl<T: AnchorAccount> core::ops::DerefMut for Optional<T> {
    fn deref_mut(&mut self) -> &mut Option<T> {
        &mut self.0
    }
}
