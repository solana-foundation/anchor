use {
    core::ops::{Deref, DerefMut},
    pinocchio::{account::AccountView, address::Address},
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program_error::ProgramError,
    crate::v2::{AnchorAccount, AnchorAccountInit, Discriminator, Owner, DISC_LEN},
};

/// Borsh-serialized account type (legacy path).
///
/// Equivalent to Anchor v1's `Account<T>`. Validates owner, checks discriminator,
/// deserializes via borsh. On `exit()`, serializes back to the data buffer.
pub struct BorshAccount<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> {
    view: AccountView,
    data: T,
    mutable: bool,
}

impl<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> BorshAccount<T> {
    fn validate_and_deserialize(view: AccountView, mutable: bool) -> Result<Self, ProgramError> {
        if !view.owned_by(&T::owner()) {
            return Err(ProgramError::IllegalOwner);
        }
        // SAFETY: slice consumed by try_from_slice (copies data out); no reference escapes.
        let data_slice = unsafe { view.borrow_unchecked() };
        if data_slice.len() < DISC_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if &data_slice[..DISC_LEN] != T::DISCRIMINATOR {
            return Err(ProgramError::InvalidAccountData);
        }
        let data = T::try_from_slice(&data_slice[DISC_LEN..])
            .map_err(|_| ProgramError::InvalidAccountData)?;
        Ok(Self { view, data, mutable })
    }
}

impl<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> AnchorAccount for BorshAccount<T> {
    type Data = T;

    fn load(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> {
        Self::validate_and_deserialize(view, false)
    }

    fn load_mut(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> {
        Self::validate_and_deserialize(view, true)
    }

    fn account(&self) -> &AccountView { &self.view }

    fn exit(&mut self) -> pinocchio::ProgramResult {
        if !self.mutable { return Ok(()); }
        let mut data_ref = self.view.try_borrow_mut()?;
        self.data.serialize(&mut &mut data_ref[DISC_LEN..])
            .map_err(|_| ProgramError::InvalidAccountData)?;
        Ok(())
    }
}

impl<T: BorshDeserialize + BorshSerialize + Owner + Discriminator + Default> AnchorAccountInit for BorshAccount<T> {
    fn init(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> {
        let mut account = Self { view, data: T::default(), mutable: true };
        let mut data_ref = account.view.try_borrow_mut()?;
        if data_ref.len() < DISC_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        data_ref[..DISC_LEN].copy_from_slice(T::DISCRIMINATOR);
        account.data.serialize(&mut &mut data_ref[DISC_LEN..])
            .map_err(|_| ProgramError::InvalidAccountData)?;
        drop(data_ref);
        Ok(account)
    }
}

impl<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> Deref for BorshAccount<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.data }
}

impl<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> DerefMut for BorshAccount<T> {
    fn deref_mut(&mut self) -> &mut T {
        if !self.mutable {
            panic!("cannot mutably deref an account loaded with load() — use #[account(mut)]");
        }
        &mut self.data
    }
}

impl<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> AsRef<AccountView> for BorshAccount<T> {
    fn as_ref(&self) -> &AccountView { &self.view }
}
