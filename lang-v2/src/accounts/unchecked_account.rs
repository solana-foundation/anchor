use {
    core::ops::Deref,
    pinocchio::{account::AccountView, address::Address},
    solana_program_error::ProgramError,
    crate::AnchorAccount,
};

pub struct UncheckedAccount { view: AccountView }

impl UncheckedAccount {
    /// Returns the account's address.
    #[inline(always)]
    pub fn address(&self) -> &Address { self.view.address() }
}

impl AnchorAccount for UncheckedAccount {
    type Data = AccountView;
    fn load(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> { Ok(Self { view }) }
    fn load_mut(view: AccountView, p: &Address) -> Result<Self, ProgramError> { Self::load(view, p) }
    fn account(&self) -> &AccountView { &self.view }
}

impl Deref for UncheckedAccount {
    type Target = AccountView;
    fn deref(&self) -> &AccountView { &self.view }
}

impl AsRef<AccountView> for UncheckedAccount {
    fn as_ref(&self) -> &AccountView { &self.view }
}

impl AsRef<Address> for UncheckedAccount {
    fn as_ref(&self) -> &Address { self.view.address() }
}
