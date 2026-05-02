use {
    crate::{AnchorAccount, Ids},
    core::{marker::PhantomData, ops::Deref},
    pinocchio::{account::AccountView, address::Address},
    solana_program_error::ProgramError,
};

/// Program account wrapper that accepts any address declared by `T::ids()`.
pub struct Interface<'info, T: Ids> {
    view: AccountView,
    _phantom: PhantomData<&'info T>,
}

impl<T: Ids> Interface<'_, T> {
    /// Returns the account's address.
    #[inline(always)]
    pub fn address(&self) -> &Address {
        self.view.address()
    }
}

impl<T: Ids> AnchorAccount for Interface<'_, T> {
    type Data = AccountView;
    type RequiredConstraints = ();

    #[inline(always)]
    fn load(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> {
        #[cfg(feature = "guardrails")]
        if !view.executable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !T::ids()
            .iter()
            .any(|id| crate::address_eq(view.address(), id))
        {
            return Err(ProgramError::IncorrectProgramId);
        }
        Ok(Self {
            view,
            _phantom: PhantomData,
        })
    }

    #[inline(always)]
    fn account(&self) -> &AccountView {
        &self.view
    }
}

impl<T: Ids> Deref for Interface<'_, T> {
    type Target = AccountView;

    #[inline(always)]
    fn deref(&self) -> &AccountView {
        &self.view
    }
}

impl<T: Ids> AsRef<AccountView> for Interface<'_, T> {
    #[inline(always)]
    fn as_ref(&self) -> &AccountView {
        &self.view
    }
}

impl<T: Ids> AsRef<Address> for Interface<'_, T> {
    #[inline(always)]
    fn as_ref(&self) -> &Address {
        self.view.address()
    }
}

#[doc(hidden)]
impl<T: Ids> crate::IdlAccountType for Interface<'_, T> {}
