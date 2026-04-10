use {
    core::ops::Deref,
    pinocchio::{account::AccountView, address::Address},
    solana_program_error::ProgramError,
    crate::{AnchorAccount, programs::System, Id},
};

pub struct SystemAccount { view: AccountView }

impl AnchorAccount for SystemAccount {
    type Data = AccountView;
    fn load(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> {
        if !view.owned_by(&System::id()) { return Err(ProgramError::IllegalOwner); }
        Ok(Self { view })
    }
    fn load_mut(view: AccountView, p: &Address) -> Result<Self, ProgramError> { Self::load(view, p) }
    fn account(&self) -> &AccountView { &self.view }
}

impl Deref for SystemAccount {
    type Target = AccountView;
    fn deref(&self) -> &AccountView { &self.view }
}

impl AsRef<AccountView> for SystemAccount {
    fn as_ref(&self) -> &AccountView { &self.view }
}

impl AsRef<Address> for SystemAccount {
    fn as_ref(&self) -> &Address { self.view.address() }
}
