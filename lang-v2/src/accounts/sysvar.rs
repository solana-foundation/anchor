use {
    core::{marker::PhantomData, ops::Deref},
    pinocchio::{
        account::AccountView,
        address::Address,
        sysvars::Sysvar as PinocchioSysvar,
    },
    solana_program_error::ProgramError,
    crate::AnchorAccount,
};

/// Trait that connects a pinocchio sysvar type to its well-known address.
///
/// Implemented for Clock, Rent, etc.
pub trait SysvarId {
    /// The sysvar's well-known account address.
    const SYSVAR_ID: Address;
}

impl SysvarId for pinocchio::sysvars::clock::Clock {
    const SYSVAR_ID: Address = pinocchio::sysvars::clock::CLOCK_ID;
}

impl SysvarId for pinocchio::sysvars::rent::Rent {
    const SYSVAR_ID: Address = pinocchio::sysvars::rent::RENT_ID;
}

/// Account wrapper for sysvars (Clock, Rent, etc.).
///
/// Validates that the passed account address matches `T::SYSVAR_ID`,
/// then deserializes the sysvar from account data via pinocchio's
/// `Sysvar::get()`.
pub struct Sysvar<T: PinocchioSysvar + SysvarId + Copy> {
    view: AccountView,
    data: T,
    _phantom: PhantomData<T>,
}

impl<T: PinocchioSysvar + SysvarId + Copy> AnchorAccount for Sysvar<T> {
    type Data = T;

    fn load(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> {
        if *view.address() != T::SYSVAR_ID {
            return Err(ProgramError::InvalidArgument);
        }
        // Use pinocchio's Sysvar::get() which reads directly from the runtime
        // via syscall, avoiding the need to deserialize from account data.
        let data = T::get().map_err(|_| ProgramError::UnsupportedSysvar)?;
        Ok(Self { view, data, _phantom: PhantomData })
    }

    fn load_mut(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        // Sysvars are read-only; load_mut behaves the same as load.
        Self::load(view, program_id)
    }

    fn account(&self) -> &AccountView { &self.view }
}

impl<T: PinocchioSysvar + SysvarId + Copy> Deref for Sysvar<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.data }
}

impl<T: PinocchioSysvar + SysvarId + Copy> AsRef<AccountView> for Sysvar<T> {
    fn as_ref(&self) -> &AccountView { &self.view }
}
