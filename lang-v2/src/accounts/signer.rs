use {
    pinocchio::{account::AccountView, address::Address},
    solana_program_error::ProgramError,
    crate::{AnchorAccount, accounts::view_wrapper_traits},
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
    fn account(&self) -> &AccountView { &self.view }
}

view_wrapper_traits!(Signer);
