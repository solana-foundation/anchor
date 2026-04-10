use {
    core::ops::{Deref, DerefMut},
    pinocchio::{
        account::{AccountView, Ref, RefMut},
        address::Address,
    },
    bytemuck::{Pod, Zeroable},
    solana_program_error::ProgramError,
    crate::{AnchorAccount, AnchorAccountInit, Discriminator, Owner, DISC_LEN},
};

/// Zero-copy account type (new default in Anchor v2).
///
/// Maps `T` directly from the account's data buffer without deserialization.
/// Uses pinocchio's borrow tracking to prevent aliasing:
/// - `load()` → immutable borrow, `Deref` only
/// - `load_mut()` → mutable borrow, `Deref` + `DerefMut`
pub struct Account<T: Pod + Zeroable + Owner + Discriminator> {
    view: AccountView,
    borrow: BorrowState<T>,
}

enum BorrowState<T> {
    Immutable { _guard: Ref<'static, [u8]>, ptr: *const T },
    Mutable { _guard: RefMut<'static, [u8]>, ptr: *mut T },
    Released,
}

impl<T: Pod + Zeroable + Owner + Discriminator> Account<T> {
    fn check_owner_and_disc(view: &AccountView, data: &[u8]) -> Result<(), ProgramError> {
        if !view.owned_by(&T::owner()) {
            return Err(ProgramError::IllegalOwner);
        }
        let min_len = DISC_LEN + core::mem::size_of::<T>();
        if data.len() < min_len {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if &data[..DISC_LEN] != T::DISCRIMINATOR {
            return Err(ProgramError::InvalidAccountData);
        }
        if (data[DISC_LEN..].as_ptr() as usize) % core::mem::align_of::<T>() != 0 {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    fn from_ref(view: AccountView) -> Result<Self, ProgramError> {
        let data_ref = view.try_borrow()?;
        Self::check_owner_and_disc(&view, &data_ref)?;
        // SAFETY: AccountView's raw pointer is valid for the entire instruction
        // lifetime (Solana runtime guarantee). The Ref guard prevents mutable
        // aliasing. We extend its lifetime to 'static because the underlying
        // data outlives any local scope within the instruction.
        let guard: Ref<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        let ptr = guard[DISC_LEN..].as_ptr() as *const T;
        Ok(Self { view, borrow: BorrowState::Immutable { _guard: guard, ptr } })
    }

    fn from_ref_mut(view: AccountView) -> Result<Self, ProgramError> {
        let mut view_mut = view;
        let data_ref = view_mut.try_borrow_mut()?;
        Self::check_owner_and_disc(&view, &data_ref)?;
        // SAFETY: Same as from_ref. RefMut provides exclusive access.
        let mut guard: RefMut<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        let ptr = guard[DISC_LEN..].as_mut_ptr() as *mut T;
        Ok(Self { view, borrow: BorrowState::Mutable { _guard: guard, ptr } })
    }
}

impl<T: Pod + Zeroable + Owner + Discriminator> AnchorAccount for Account<T> {
    type Data = T;

    fn load(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> {
        Self::from_ref(view)
    }

    fn load_mut(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> {
        Self::from_ref_mut(view)
    }

    fn account(&self) -> &AccountView { &self.view }

    fn close(&mut self, mut destination: AccountView) -> pinocchio::ProgramResult {
        // Release the borrow guard before closing so pinocchio's close() can proceed
        self.borrow = BorrowState::Released;
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
}

impl<T: Pod + Zeroable + Owner + Discriminator> AnchorAccountInit for Account<T> {
    fn init(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> {
        let mut view_mut = view;
        let mut data_ref = view_mut.try_borrow_mut()?;
        let min_len = DISC_LEN + core::mem::size_of::<T>();
        if data_ref.len() < min_len {
            return Err(ProgramError::AccountDataTooSmall);
        }
        data_ref[..DISC_LEN].copy_from_slice(T::DISCRIMINATOR);
        data_ref[DISC_LEN..DISC_LEN + core::mem::size_of::<T>()].fill(0);
        // Reuse the existing RefMut — no double borrow
        // SAFETY: same lifetime reasoning as from_ref_mut
        let mut guard: RefMut<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        let ptr = guard[DISC_LEN..].as_mut_ptr() as *mut T;
        Ok(Self { view, borrow: BorrowState::Mutable { _guard: guard, ptr } })
    }
}

impl<T: Pod + Zeroable + Owner + Discriminator> Deref for Account<T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: ptr was validated during load. Guard is held.
        match &self.borrow {
            BorrowState::Immutable { ptr, .. } => unsafe { &**ptr },
            BorrowState::Mutable { ptr, .. } => unsafe { &*(*ptr as *const T) },
            BorrowState::Released => panic!("account borrow released (closed)"),
        }
    }
}

impl<T: Pod + Zeroable + Owner + Discriminator> DerefMut for Account<T> {
    fn deref_mut(&mut self) -> &mut T {
        match &mut self.borrow {
            BorrowState::Mutable { ptr, .. } => unsafe { &mut **ptr },
            BorrowState::Immutable { .. } => panic!("use #[account(mut)] for mutable access"),
            BorrowState::Released => panic!("account borrow released (closed)"),
        }
    }
}

impl<T: Pod + Zeroable + Owner + Discriminator> AsRef<AccountView> for Account<T> {
    fn as_ref(&self) -> &AccountView { &self.view }
}
