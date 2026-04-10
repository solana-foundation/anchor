use {
    core::ops::{Deref, DerefMut},
    pinocchio::{
        account::{AccountView, Ref, RefMut},
        address::Address,
    },
    bytemuck::{Pod, Zeroable},
    solana_program_error::ProgramError,
    crate::{AnchorAccount, AnchorAccountInit, Discriminator, Id, Owner},
};

/// Controls how `Account<T>` validates and maps account data.
///
/// Types marked with `#[account]` get this automatically via the blanket impl
/// over `Owner + Discriminator`. External types (e.g. SPL TokenAccount) implement
/// this directly with custom validation (exact length checks, no discriminator).
pub trait AccountValidate {
    /// Validate the raw account data before mapping.
    fn validate(view: &AccountView, data: &[u8]) -> Result<(), ProgramError>;

    /// Byte offset where `T`'s data starts in the account buffer.
    /// Anchor accounts: discriminator length. External accounts: 0.
    fn data_offset() -> usize;
}

/// Blanket impl: every `#[account]` type (Owner + Discriminator) gets standard
/// Anchor validation — owner check, discriminator check, length check.
impl<T: Owner + Discriminator> AccountValidate for T {
    fn validate(view: &AccountView, data: &[u8]) -> Result<(), ProgramError> {
        if view.lamports() == 0 && view.owned_by(&crate::programs::System::id()) {
            return Err(ProgramError::UninitializedAccount);
        }
        if !view.owned_by(&T::owner()) {
            return Err(ProgramError::IllegalOwner);
        }
        let disc = T::DISCRIMINATOR;
        let min_len = disc.len() + core::mem::size_of::<T>();
        if data.len() < min_len {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if &data[..disc.len()] != disc {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    fn data_offset() -> usize {
        T::DISCRIMINATOR.len()
    }
}

/// Defines how to create and initialize an account type via CPI.
///
/// The `Params` struct acts as a compile-time hashmap: its fields are the valid
/// init parameter keys. The macro constructs it from namespaced constraints
/// (`token::mint = mint` → `params.mint = Some(mint.account())`).
/// Missing fields get `None` from `Default`. Unknown fields → compile error.
///
/// The `create_and_initialize` method handles both account creation
/// (system program CPI) and type-specific initialization (e.g. token program CPI).
/// This keeps all account creation logic in the type, not in the macro.
pub trait AccountInitialize {
    type Params<'a>: Default;

    /// Create the account and initialize it.
    /// `payer` funds the account, `account` is the target, `program_id` is the
    /// owning program. For PDA accounts, `signer_seeds` contains the seeds + bump.
    fn create_and_initialize<'a>(
        payer: &AccountView,
        account: &AccountView,
        program_id: &Address,
        params: &Self::Params<'a>,
        signer_seeds: Option<&[&[u8]]>,
    ) -> Result<(), ProgramError>;
}

/// Zero-copy account type (new default in Anchor v2).
///
/// Maps `T` directly from the account's data buffer without deserialization.
/// Uses pinocchio's borrow tracking to prevent aliasing:
/// - `load()` → immutable borrow, `Deref` only
/// - `load_mut()` → mutable borrow, `Deref` + `DerefMut`
pub struct Account<T: Pod + Zeroable + AccountValidate> {
    view: AccountView,
    borrow: BorrowState<T>,
}

enum BorrowState<T> {
    Immutable { _guard: Ref<'static, [u8]>, ptr: *const T },
    Mutable { _guard: RefMut<'static, [u8]>, ptr: *mut T },
    Released,
}

impl<T: Pod + Zeroable + AccountValidate> Account<T> {
    /// Release the data borrow guard so the underlying `AccountView` can be
    /// passed to CPI calls that check `is_borrowed()`. After calling this,
    /// `Deref` / `DerefMut` will panic until `reacquire_borrow()` is called.
    pub fn release_borrow(&mut self) {
        self.borrow = BorrowState::Released;
    }

    /// Re-acquire an immutable borrow after a `release_borrow()` + CPI.
    /// This allows reading updated account data (e.g. checking balances
    /// after a token transfer).
    pub fn reacquire_borrow(&mut self) -> core::result::Result<(), ProgramError> {
        let data_ref = self.view.try_borrow()?;
        let offset = T::data_offset();
        let guard: Ref<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        let ptr = guard[offset..].as_ptr() as *const T;
        self.borrow = BorrowState::Immutable { _guard: guard, ptr };
        Ok(())
    }

    /// Re-acquire a mutable borrow after a `release_borrow()` + CPI.
    /// This allows reading and writing updated account data.
    pub fn reacquire_borrow_mut(&mut self) -> core::result::Result<(), ProgramError> {
        let mut view_mut = self.view;
        let data_ref = view_mut.try_borrow_mut()?;
        let offset = T::data_offset();
        let mut guard: RefMut<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        let ptr = guard[offset..].as_mut_ptr() as *mut T;
        self.borrow = BorrowState::Mutable { _guard: guard, ptr };
        Ok(())
    }

    fn from_ref(view: AccountView) -> Result<Self, ProgramError> {
        let data_ref = view.try_borrow()?;
        T::validate(&view, &data_ref)?;
        let offset = T::data_offset();
        // SAFETY: AccountView's raw pointer is valid for the entire instruction
        // lifetime (Solana runtime guarantee). The Ref guard prevents mutable
        // aliasing. We extend its lifetime to 'static because the underlying
        // data outlives any local scope within the instruction.
        let guard: Ref<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        let ptr = guard[offset..].as_ptr() as *const T;
        Ok(Self { view, borrow: BorrowState::Immutable { _guard: guard, ptr } })
    }

    fn from_ref_mut(view: AccountView) -> Result<Self, ProgramError> {
        let mut view_mut = view;
        let data_ref = view_mut.try_borrow_mut()?;
        T::validate(&view, &data_ref)?;
        let offset = T::data_offset();
        // SAFETY: Same as from_ref. RefMut provides exclusive access.
        let mut guard: RefMut<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        let ptr = guard[offset..].as_mut_ptr() as *mut T;
        Ok(Self { view, borrow: BorrowState::Mutable { _guard: guard, ptr } })
    }
}

impl<T: Pod + Zeroable + AccountValidate> AnchorAccount for Account<T> {
    type Data = T;

    fn load(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> {
        Self::from_ref(view)
    }

    fn load_mut(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> {
        if !view.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        Self::from_ref_mut(view)
    }

    fn account(&self) -> &AccountView { &self.view }

    fn close(&mut self, mut destination: AccountView) -> pinocchio::ProgramResult {
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

impl<T: Pod + Zeroable + AccountValidate + Discriminator> AnchorAccountInit for Account<T> {
    fn init(view: AccountView, _program_id: &Address) -> Result<Self, ProgramError> {
        let mut view_mut = view;
        let mut data_ref = view_mut.try_borrow_mut()?;
        let disc = T::DISCRIMINATOR;
        let min_len = disc.len() + core::mem::size_of::<T>();
        if data_ref.len() < min_len {
            return Err(ProgramError::AccountDataTooSmall);
        }
        data_ref[..disc.len()].copy_from_slice(disc);
        data_ref[disc.len()..min_len].fill(0);
        let offset = disc.len();
        // SAFETY: same lifetime reasoning as from_ref_mut
        let mut guard: RefMut<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        let ptr = guard[offset..].as_mut_ptr() as *mut T;
        Ok(Self { view, borrow: BorrowState::Mutable { _guard: guard, ptr } })
    }
}

impl<T: Pod + Zeroable + AccountValidate> Deref for Account<T> {
    type Target = T;
    fn deref(&self) -> &T {
        match &self.borrow {
            BorrowState::Immutable { ptr, .. } => unsafe { &**ptr },
            BorrowState::Mutable { ptr, .. } => unsafe { &*(*ptr as *const T) },
            BorrowState::Released => panic!("account borrow released (closed)"),
        }
    }
}

impl<T: Pod + Zeroable + AccountValidate> DerefMut for Account<T> {
    fn deref_mut(&mut self) -> &mut T {
        match &mut self.borrow {
            BorrowState::Mutable { ptr, .. } => unsafe { &mut **ptr },
            BorrowState::Immutable { .. } => panic!("use #[account(mut)] for mutable access"),
            BorrowState::Released => panic!("account borrow released (closed)"),
        }
    }
}

impl<T: Pod + Zeroable + AccountValidate> AsRef<AccountView> for Account<T> {
    fn as_ref(&self) -> &AccountView { &self.view }
}

impl<T: Pod + Zeroable + AccountValidate> AsRef<Address> for Account<T> {
    fn as_ref(&self) -> &Address { self.view.address() }
}
