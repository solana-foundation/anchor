use {
    core::ops::Deref,
    pinocchio::{account::AccountView, address::Address},
    solana_program_error::ProgramError,
    crate::v2::AnchorAccount,
};

pub struct Signer { view: AccountView }

impl AnchorAccount for Signer {
    type Data = AccountView;
    fn load(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> {
        if !view.is_signer() { return Err(ProgramError::MissingRequiredSignature); }
        Ok(Self { view })
    }
    fn load_mut(view: AccountView, p: &Address) -> Result<Self, ProgramError> { Self::load(view, p) }
    fn account(&self) -> &AccountView { &self.view }
}

impl Deref for Signer {
    type Target = AccountView;
    fn deref(&self) -> &AccountView { &self.view }
}

impl AsRef<AccountView> for Signer {
    fn as_ref(&self) -> &AccountView { &self.view }
}
