//! Rent sysvar: use Agave’s [`solana_rent::Rent`] math (includes `exemption_threshold`). Pinocchio’s
//! `Rent` type omits that field and underfunds `#[account(init)]`. Wrapped as a newtype so we can
//! implement Pinocchio’s [`Sysvar`](pinocchio::sysvars::Sysvar) for [`crate::accounts::sysvar::Sysvar`].
//!
//! Both [`solana_sysvar::Sysvar`] and Pinocchio’s `Sysvar` call the same [`solana_sysvar::rent::Rent::get`];
//! there is no second layout or syscall path—`exemption_threshold` always comes from the Agave rent sysvar.

pub use solana_sysvar::rent::{check_id, id, ID};
use std::ops::Deref;

#[derive(Clone, Debug, PartialEq, Default)]
#[repr(transparent)]
pub struct Rent(pub solana_sysvar::rent::Rent);

impl Deref for Rent {
    type Target = solana_sysvar::rent::Rent;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl solana_sysvar::Sysvar for Rent {
    fn get() -> Result<Self, solana_program_error::ProgramError> {
        Ok(Self(solana_sysvar::rent::Rent::get()?))
    }
}

impl pinocchio::sysvars::Sysvar for Rent {
    fn get() -> Result<Self, pinocchio::error::ProgramError> {
        <Self as solana_sysvar::Sysvar>::get()
    }
}
