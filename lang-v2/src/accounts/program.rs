use {
    core::{marker::PhantomData, ops::Deref},
    pinocchio::{account::AccountView, address::Address},
    solana_program_error::ProgramError,
    crate::{AnchorAccount, Id},
};

pub struct Program<T: Id> { view: AccountView, _phantom: PhantomData<T> }

impl<T: Id> Program<T> {
    /// Returns the account's address.
    #[inline(always)]
    pub fn address(&self) -> &Address { self.view.address() }
}

impl<T: Id> AnchorAccount for Program<T> {
    type Data = AccountView;
    #[inline(always)]
    fn load(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> {
        #[cfg(feature = "guardrails")]
        if !view.executable() { return Err(ProgramError::InvalidAccountData); }
        let id = T::id();
        if !crate::address_eq(view.address(), &id) {
            return Err(ProgramError::IncorrectProgramId);
        }
        Ok(Self { view, _phantom: PhantomData })
    }
    #[inline(always)]
    fn account(&self) -> &AccountView { &self.view }
}

impl<T: Id> Deref for Program<T> {
    type Target = AccountView;
    fn deref(&self) -> &AccountView { &self.view }
}

impl<T: Id> AsRef<AccountView> for Program<T> {
    fn as_ref(&self) -> &AccountView { &self.view }
}

impl<T: Id> AsRef<Address> for Program<T> {
    fn as_ref(&self) -> &Address { self.view.address() }
}
