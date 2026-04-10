use {
    core::{marker::PhantomData, ops::Deref},
    pinocchio::{account::AccountView, address::Address},
    solana_program_error::ProgramError,
    crate::v2::{AnchorAccount, Id},
};

pub struct Program<T: Id> { view: AccountView, _phantom: PhantomData<T> }

impl<T: Id> AnchorAccount for Program<T> {
    type Data = AccountView;
    fn load(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> {
        if !view.executable() { return Err(ProgramError::InvalidAccountData); }
        if *view.address() != T::id() { return Err(ProgramError::IncorrectProgramId); }
        Ok(Self { view, _phantom: PhantomData })
    }
    fn load_mut(view: AccountView, p: &Address) -> Result<Self, ProgramError> { Self::load(view, p) }
    fn account(&self) -> &AccountView { &self.view }
}

impl<T: Id> Deref for Program<T> {
    type Target = AccountView;
    fn deref(&self) -> &AccountView { &self.view }
}

impl<T: Id> AsRef<AccountView> for Program<T> {
    fn as_ref(&self) -> &AccountView { &self.view }
}
