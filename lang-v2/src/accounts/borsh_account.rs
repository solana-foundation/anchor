use {
    core::ops::{Deref, DerefMut},
    pinocchio::account::{AccountView, Ref, RefMut},
    pinocchio::address::Address,
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program_error::ProgramError,
    crate::{AnchorAccount, Discriminator, Id, Owner},
};

/// Discriminator length in bytes. All `#[account]` types use an 8-byte
/// discriminator; borsh accounts prefix their data with it.
const DISC_LEN: usize = 8;

/// Borsh-serialized account type.
///
/// Validates owner, checks discriminator, deserializes via borsh.
/// Holds a pinocchio borrow guard to prevent duplicate mutable accounts:
/// - `load()` takes an immutable borrow (blocks subsequent `load_mut` on same account)
/// - `load_mut()` takes a mutable borrow (blocks any other borrow on same account)
/// - `exit()` serializes through the held `RefMut`
pub struct BorshAccount<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> {
    view: AccountView,
    data: T,
    borrow: BorshBorrow,
}

enum BorshBorrow {
    Immutable { _guard: Ref<'static, [u8]> },
    Mutable { guard: RefMut<'static, [u8]> },
    Released,
}

impl<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> BorshAccount<T> {
    fn validate_and_load(view: AccountView, data: &[u8], program_id: &Address) -> Result<T, ProgramError> {
        if view.lamports() == 0 && view.owned_by(&crate::programs::System::id()) {
            return Err(ProgramError::UninitializedAccount);
        }
        if !view.owned_by(&T::owner(program_id)) {
            return Err(ProgramError::IllegalOwner);
        }
        if data.len() < DISC_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if &data[..DISC_LEN] != T::DISCRIMINATOR {
            return Err(ProgramError::InvalidAccountData);
        }
        T::deserialize(&mut &data[DISC_LEN..])
            .map_err(|_| ProgramError::InvalidAccountData)
    }
}

impl<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> AnchorAccount for BorshAccount<T> {
    type Data = T;

    fn load(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        let data_ref = view.try_borrow()?;
        let data = Self::validate_and_load(view, &data_ref, program_id)?;
        // SAFETY: AccountView's raw pointer is valid for the entire instruction
        // lifetime (Solana runtime guarantee). We hold the Ref to prevent
        // subsequent mutable borrows on the same account (duplicate detection).
        let guard: Ref<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        Ok(Self { view, data, borrow: BorshBorrow::Immutable { _guard: guard } })
    }

    fn load_mut(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        if !view.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        let mut view_mut = view;
        let data_ref = view_mut.try_borrow_mut()?;
        let data = Self::validate_and_load(view, &data_ref, program_id)?;
        // SAFETY: Same as load(). RefMut provides exclusive access and prevents
        // any other borrow on the same account.
        let guard: RefMut<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        Ok(Self { view, data, borrow: BorshBorrow::Mutable { guard } })
    }

    fn account(&self) -> &AccountView { &self.view }

    fn close(&mut self, mut destination: AccountView) -> pinocchio::ProgramResult {
        // Release the borrow guard before closing so pinocchio's close() can proceed
        self.borrow = BorshBorrow::Released;
        let mut self_view = self.view;
        let dest_lamports = destination
            .lamports()
            .checked_add(self_view.lamports())
            .ok_or(ProgramError::ArithmeticOverflow)?;
        destination.set_lamports(dest_lamports);
        self_view.set_lamports(0);
        self_view.close()?;
        Ok(())
    }

    fn exit(&mut self) -> pinocchio::ProgramResult {
        // Skip serialization if account was closed (lamports == 0, reassigned to system program).
        if self.view.lamports() == 0 {
            return Ok(());
        }
        // Write through the held RefMut — no need to re-acquire the borrow
        if let BorshBorrow::Mutable { ref mut guard } = self.borrow {
            self.data.serialize(&mut &mut guard[DISC_LEN..])
                .map_err(|_| ProgramError::InvalidAccountData)?;
        }
        Ok(())
    }
}

impl<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> Deref for BorshAccount<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.data }
}

impl<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> DerefMut for BorshAccount<T> {
    fn deref_mut(&mut self) -> &mut T {
        match &self.borrow {
            BorshBorrow::Mutable { .. } => &mut self.data,
            BorshBorrow::Immutable { .. } => panic!("use #[account(mut)] for mutable access"),
            BorshBorrow::Released => panic!("account borrow released (closed)"),
        }
    }
}

impl<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> AsRef<AccountView> for BorshAccount<T> {
    fn as_ref(&self) -> &AccountView { &self.view }
}
