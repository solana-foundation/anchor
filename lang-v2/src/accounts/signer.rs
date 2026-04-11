use {
    core::ops::Deref,
    pinocchio::{account::AccountView, address::Address},
    solana_program_error::ProgramError,
    crate::AnchorAccount,
};

pub struct Signer { view: AccountView }

impl Signer {
    /// Returns the account's address.
    #[inline(always)]
    pub fn address(&self) -> &Address { self.view.address() }
}

impl AnchorAccount for Signer {
    type Data = AccountView;
    #[inline(always)]
    fn load(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> {
        if !view.is_signer() { return Err(ProgramError::MissingRequiredSignature); }
        Ok(Self { view })
    }
    #[inline(always)]
    fn load_mut(view: AccountView, p: &Address) -> Result<Self, ProgramError> { Self::load(view, p) }
    #[inline(always)]
    fn account(&self) -> &AccountView { &self.view }
}

impl Deref for Signer {
    type Target = AccountView;
    fn deref(&self) -> &AccountView { &self.view }
}

impl AsRef<AccountView> for Signer {
    fn as_ref(&self) -> &AccountView { &self.view }
}

impl AsRef<Address> for Signer {
    fn as_ref(&self) -> &Address { self.view.address() }
}
