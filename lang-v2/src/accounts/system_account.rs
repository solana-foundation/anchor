use {
    crate::{accounts::view_wrapper_traits, programs::System, AnchorAccount, Id},
    pinocchio::{account::AccountView, address::Address},
    solana_program_error::ProgramError,
};

pub struct SystemAccount {
    view: AccountView,
}

impl SystemAccount {
    /// Returns the account's address.
    #[inline(always)]
    pub fn address(&self) -> &Address {
        self.view.address()
    }
}

impl AnchorAccount for SystemAccount {
    type Data = AccountView;
    #[inline(always)]
    fn load(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> {
        if !view.owned_by(&System::id()) {
            return Err(ProgramError::IllegalOwner);
        }
        Ok(Self { view })
    }
    #[inline(always)]
    fn account(&self) -> &AccountView {
        &self.view
    }
}

view_wrapper_traits!(SystemAccount);

#[cfg(feature = "idl-build")]
impl crate::IdlAccountType for SystemAccount {}
