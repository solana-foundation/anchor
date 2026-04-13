extern crate alloc;

use {
    alloc::boxed::Box,
    pinocchio::{
        account::AccountView,
        address::Address,
    },
    solana_program_error::ProgramError,
    crate::AnchorAccount,
};

impl<T: AnchorAccount> AnchorAccount for Box<T> {
    type Data = T;

    fn load(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        T::load(view, program_id).map(Box::new)
    }

    fn load_mut(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        T::load_mut(view, program_id).map(Box::new)
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
