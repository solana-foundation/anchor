use {
    core::ops::{Deref, DerefMut},
    pinocchio::{
        account::{AccountView, Ref, RefMut},
        address::Address,
    },
    bytemuck::{Pod, Zeroable},
    solana_program_error::ProgramError,
    crate::{AnchorAccount, Discriminator, Id, Owner},
};

/// Controls how `Account<T>` validates and maps account data.
///
/// Types marked with `#[account]` get this automatically via the blanket impl
/// over `Owner + Discriminator`. External types (e.g. SPL TokenAccount) implement
/// this directly with custom validation (exact length checks, no discriminator).
pub trait AccountValidate {
    /// Validate the raw account data before mapping.
    /// `program_id` is available for owner checks via `Owner::owner(program_id)`.
    fn validate(view: &AccountView, data: &[u8], program_id: &Address) -> Result<(), ProgramError>;

    /// Byte offset where `T`'s data starts in the account buffer.
    /// Anchor accounts: discriminator length. External accounts: 0.
    fn data_offset() -> usize;
}

/// Blanket impl: every `#[account]` type (Owner + Discriminator) gets standard
/// Anchor validation — owner check via Owner::owner(program_id).
impl<T: Owner + Discriminator> AccountValidate for T {
    #[inline(always)]
    fn validate(view: &AccountView, data: &[u8], program_id: &Address) -> Result<(), ProgramError> {
        // Hot path: a single owner check. The "uninitialized placeholder"
        // disambiguation lives in `cold_owner_error` — placeholder accounts
        // (lamports=0, owner=system) always fail this owner check too,
        // since `T::owner(program_id)` is the user's program, never system.
        // The cold helper turns the failure into a more specific error
        // code without paying the extra loads on the validation-passes path.
        if !view.owned_by(&T::owner(program_id)) {
            return Err(cold_owner_error(view));
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

    #[inline(always)]
    fn data_offset() -> usize {
        T::DISCRIMINATOR.len()
    }
}

/// Disambiguation for failed owner checks. The hot path branches here when
/// the owner doesn't match `T::owner(program_id)`; this helper distinguishes
/// the two failure modes (uninitialized placeholder vs. genuine wrong owner)
/// so the caller gets a precise error code without the disambiguation cost
/// being paid on every successful load.
///
/// Marked `#[inline(always)]` (not `#[cold] #[inline(never)]`) after
/// benchmarking four variants on clear-msig-anchor: `#[cold]` adds ~0.5 KB
/// binary and nets zero CU improvement on SBPF — the branch-prediction /
/// code-layout reasons it exists for on x86/ARM don't apply here (linear
/// program image, no I-cache locality benefit, no hardware predictor). Keep
/// the helper (it centralises the two-line disambiguation and saves a few
/// bytes per typed-account load site), but drop the cold annotation.
#[inline(always)]
pub(super) fn cold_owner_error(view: &AccountView) -> ProgramError {
    if view.lamports() == 0 && view.owned_by(&crate::programs::System::id()) {
        ProgramError::UninitializedAccount
    } else {
        ProgramError::IllegalOwner
    }
}

/// Error constructor for the read-only-write rejection in `load_mut`.
/// Same `#[inline(always)]` rationale as `cold_owner_error`.
#[inline(always)]
pub(super) fn cold_not_writable() -> ProgramError {
    ProgramError::InvalidAccountData
}

/// Defines how to create and initialize an account type via CPI.
///
/// The `Params` struct acts as a compile-time hashmap: its fields are the valid
/// init parameter keys. The macro constructs it from namespaced constraints
/// (`token::mint = mint` → `params.mint = Some(mint.account())`).
/// Missing fields get `None` from `Default`. Unknown fields → compile error.
///
/// Blanket impl for `Owner + Discriminator` handles Anchor program accounts
/// (create account + write discriminator). External types (TokenAccount, Mint)
/// implement this directly with custom CPI logic.
pub trait AccountInitialize {
    type Params<'a>: Default;

    /// Create the account and initialize it.
    /// `space` is the total account data size (including discriminator).
    fn create_and_initialize<'a>(
        payer: &AccountView,
        account: &AccountView,
        space: usize,
        program_id: &Address,
        params: &Self::Params<'a>,
        signer_seeds: Option<&[&[u8]]>,
    ) -> Result<(), ProgramError>;
}

/// Blanket impl: all Anchor program accounts (Owner + Discriminator) get default
/// init behavior — create_account + write discriminator. The remaining data is
/// zeroed by create_account, which is valid for both Pod and borsh types
/// (borsh zero bytes = default values for integers, empty strings/vecs).
impl<T: Owner + Discriminator> AccountInitialize for T {
    type Params<'a> = ();

    #[inline(always)]
    fn create_and_initialize<'a>(
        payer: &AccountView,
        account: &AccountView,
        space: usize,
        program_id: &Address,
        _params: &(),
        signer_seeds: Option<&[&[u8]]>,
    ) -> Result<(), ProgramError> {
        let disc: &[u8; 8] = T::DISCRIMINATOR
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?;
        match signer_seeds {
            Some(seeds) => crate::create_account_signed(payer, account, space, program_id, seeds)?,
            None => crate::create_account(payer, account, space, program_id)?,
        }
        // Panic-free disc write: `first_chunk_mut::<8>` returns `Option`, so
        // LLVM emits a single qword store on the happy path and a plain
        // ProgramError return on failure — no `slice_end_index_len_fail` and
        // no core::fmt panic machinery pulled into the binary.
        let mut account_view = *account;
        let data = unsafe { account_view.borrow_unchecked_mut() };
        match data.first_chunk_mut::<8>() {
            Some(dst) => *dst = *disc,
            None => return Err(ProgramError::AccountDataTooSmall),
        }
        Ok(())
    }
}

/// Zero-copy account type (new default in Anchor v2).
///
/// Maps `T` directly from the account's data buffer without deserialization.
/// Uses pinocchio's borrow tracking to prevent aliasing:
/// - `load()` → immutable borrow, `Deref` only
/// - `load_mut()` → mutable borrow, `Deref` + `DerefMut`
///
/// # Internals
///
/// Holds a cached typed pointer plus the pinocchio borrow guard. The
/// guard's existence is what prevents aliasing — pinocchio's refcount
/// rejects further `try_borrow*` calls while it's alive. Field access
/// goes through the cached pointer with no per-access dispatch in the
/// common case.
///
/// The optional `guardrails` feature (default-on) adds a runtime check
/// on `Deref`/`DerefMut` that catches:
/// - Use-after-`release_borrow()` (caller forgot to `reacquire_borrow_mut`)
/// - Use-after-`close()`
/// - `DerefMut` on a read-only-loaded account (missing `#[account(mut)]`)
///
/// These checks are panics with descriptive messages. Disabling
/// `guardrails` saves ~20 CU per access, but unchecked misuse is
/// silent (UB at the program level — usually wrong reads or
/// runtime-rejected writes).
pub struct Account<T: Pod + Zeroable + AccountValidate> {
    view: AccountView,
    /// Cached typed pointer into the account data. Valid while `guard`
    /// is `Some`. After `release_borrow()` or `close()`, the pointer
    /// is stale and must not be dereferenced (panics with `guardrails`).
    ptr: *mut T,
    /// The active borrow guard. `Some` while the account is borrowed,
    /// `None` after `release_borrow()` or `close()`. The variant
    /// (Immutable vs Mutable) determines whether `DerefMut` is allowed.
    guard: Option<BorrowGuard>,
}

/// Holds the live pinocchio borrow guard for an `Account<T>`. The
/// guards are kept around for their Drop side effect (releasing the
/// underlying borrow refcount), not their data — `Account<T>` reads
/// and writes through its cached `ptr` field instead. The variant
/// distinguishes whether `DerefMut` is allowed.
#[allow(dead_code)]
enum BorrowGuard {
    Immutable(Ref<'static, [u8]>),
    Mutable(RefMut<'static, [u8]>),
}

impl<T: Pod + Zeroable + AccountValidate> Account<T> {
    /// Returns the account's address. Always safe regardless of borrow state.
    #[inline(always)]
    pub fn address(&self) -> &Address { self.view.address() }

    /// Release the data borrow guard so the underlying `AccountView` can be
    /// passed to CPI calls that check `is_borrowed()`. After calling this,
    /// `Deref`/`DerefMut` will panic (with `guardrails`) until
    /// `reacquire_borrow_mut()` is called.
    #[inline]
    pub fn release_borrow(&mut self) {
        self.guard = None;
        // ptr is now stale; further deref is caller error.
    }

    /// Re-acquire an immutable borrow after a `release_borrow()` + CPI.
    /// This allows reading updated account data (e.g. checking balances
    /// after a token transfer).
    pub fn reacquire_borrow(&mut self) -> core::result::Result<(), ProgramError> {
        let data_ref = self.view.try_borrow()?;
        let offset = T::data_offset();
        // SAFETY: see from_ref.
        let guard: Ref<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        self.ptr = guard[offset..].as_ptr() as *mut T;
        self.guard = Some(BorrowGuard::Immutable(guard));
        Ok(())
    }

    /// Re-acquire a mutable borrow after a `release_borrow()` + CPI.
    /// This allows reading and writing updated account data.
    pub fn reacquire_borrow_mut(&mut self) -> core::result::Result<(), ProgramError> {
        let mut view_mut = self.view;
        let data_ref = view_mut.try_borrow_mut()?;
        let offset = T::data_offset();
        let mut guard: RefMut<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        self.ptr = guard[offset..].as_mut_ptr() as *mut T;
        self.guard = Some(BorrowGuard::Mutable(guard));
        Ok(())
    }

    #[inline(always)]
    fn from_ref(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        let data_ref = view.try_borrow()?;
        T::validate(&view, &data_ref, program_id)?;
        let offset = T::data_offset();
        // SAFETY: AccountView's raw pointer is valid for the entire instruction
        // lifetime. The Ref/RefMut guard prevents aliasing. We extend its
        // lifetime to 'static because the underlying data outlives any local
        // scope within the instruction.
        let guard: Ref<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        let ptr = guard[offset..].as_ptr() as *mut T;
        Ok(Self { view, ptr, guard: Some(BorrowGuard::Immutable(guard)) })
    }

    #[inline(always)]
    fn from_ref_mut(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        let mut view_mut = view;
        let data_ref = view_mut.try_borrow_mut()?;
        T::validate(&view, &data_ref, program_id)?;
        let offset = T::data_offset();
        let mut guard: RefMut<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        let ptr = guard[offset..].as_mut_ptr() as *mut T;
        Ok(Self { view, ptr, guard: Some(BorrowGuard::Mutable(guard)) })
    }
}

impl<T: Pod + Zeroable + AccountValidate> AnchorAccount for Account<T> {
    type Data = T;

    #[inline(always)]
    fn load(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        Self::from_ref(view, program_id)
    }

    #[inline(always)]
    fn load_mut(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        if !view.is_writable() {
            return Err(cold_not_writable());
        }
        Self::from_ref_mut(view, program_id)
    }

    #[inline(always)]
    fn account(&self) -> &AccountView { &self.view }

    fn close(&mut self, mut destination: AccountView) -> pinocchio::ProgramResult {
        // Drop the borrow guard before mutating the underlying account
        // state, so any nested helpers can re-borrow it cleanly.
        self.guard = None;
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

impl<T: Pod + Zeroable + AccountValidate> Deref for Account<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        // Reading is allowed under either guard variant; only catch
        // use-after-release/close.
        #[cfg(feature = "guardrails")]
        if self.guard.is_none() {
            panic!(
                "Account<T> dereferenced after release_borrow() or close(). \
                 Call reacquire_borrow_mut() before accessing fields again."
            );
        }
        // SAFETY: while `guard` is `Some`, pinocchio's refcount holds the
        // borrow open and prevents aliasing. With `guardrails` disabled,
        // the caller is responsible for not dereferencing after release.
        unsafe { &*self.ptr }
    }
}

impl<T: Pod + Zeroable + AccountValidate> DerefMut for Account<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        // Writing requires a mutable guard. The Solana runtime would
        // reject the tx if we wrote through an immutably-borrowed
        // account, so this catches the misuse early with a clearer
        // error than the runtime's generic "ReadonlyDataModified".
        #[cfg(feature = "guardrails")]
        match &self.guard {
            None => panic!(
                "Account<T> mutably dereferenced after release_borrow() or close(). \
                 Call reacquire_borrow_mut() before accessing fields again."
            ),
            Some(BorrowGuard::Immutable(_)) => panic!(
                "Account<T> mutably dereferenced but loaded read-only. \
                 Add #[account(mut)] to your accounts struct."
            ),
            Some(BorrowGuard::Mutable(_)) => {}
        }
        // SAFETY: under a Mutable guard, no other live borrow exists.
        // The Rust borrow checker (we hold &mut self) ensures uniqueness.
        unsafe { &mut *self.ptr }
    }
}

impl<T: Pod + Zeroable + AccountValidate> AsRef<AccountView> for Account<T> {
    fn as_ref(&self) -> &AccountView { &self.view }
}

impl<T: Pod + Zeroable + AccountValidate> AsRef<Address> for Account<T> {
    fn as_ref(&self) -> &Address { self.view.address() }
}
