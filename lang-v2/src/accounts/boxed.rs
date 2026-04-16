extern crate alloc;

use {
    crate::AnchorAccount,
    alloc::boxed::Box,
    pinocchio::{account::AccountView, address::Address},
    solana_program_error::ProgramError,
};

impl<T: AnchorAccount> AnchorAccount for Box<T> {
    type Data = T;
    const MIN_DATA_LEN: usize = T::MIN_DATA_LEN;

    fn load(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        T::load(view, program_id).map(Box::new)
    }

    /// # Safety
    ///
    /// See [`AnchorAccount::load_mut`] — caller must ensure no other live
    /// `&mut` to the same account data exists.
    unsafe fn load_mut(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        T::load_mut(view, program_id).map(Box::new)
    }

    /// # Safety
    ///
    /// See [`AnchorAccount::load_mut_after_init`] — caller must ensure no
    /// other live `&mut` to the same account data exists.
    unsafe fn load_mut_after_init(
        view: AccountView,
        program_id: &Address,
    ) -> Result<Self, ProgramError> {
        T::load_mut_after_init(view, program_id).map(Box::new)
    }

    fn account(&self) -> &AccountView {
        (**self).account()
    }

    fn exit(&mut self) -> pinocchio::ProgramResult {
        (**self).exit()
    }

    fn close(&mut self, destination: AccountView) -> pinocchio::ProgramResult {
        (**self).close(destination)
    }
}

#[cfg(feature = "idl-build")]
impl<T: crate::IdlAccountType> crate::IdlAccountType for Box<T> {
    const __IDL_TYPE: Option<&'static str> = T::__IDL_TYPE;
}
