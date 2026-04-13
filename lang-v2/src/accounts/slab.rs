use {
    core::{
        marker::PhantomData,
        ops::{Deref, DerefMut, Index, IndexMut},
    },
    pinocchio::{
        account::{AccountView, Ref, RefMut},
        address::Address,
    },
    bytemuck::{Pod, Zeroable},
    solana_program_error::ProgramError,
    crate::{AnchorAccount, Discriminator, Id, Owner},
};

// ---------------------------------------------------------------------------
// AccountValidate / AccountInitialize traits (moved here from the old
// account.rs). External types like SPL TokenAccount / Mint implement
// these directly; every `#[account]` struct gets the default impls via the
// blanket impls below.
// ---------------------------------------------------------------------------

/// Controls how `Slab<H, T>` (and therefore the `Account<T>` alias)
/// validates and maps account data.
///
/// Types marked with `#[account]` get this automatically via the blanket impl
/// over `Owner + Discriminator`. External types (e.g. SPL `TokenAccount`)
/// implement this directly with custom validation (exact length checks, no
/// discriminator).
pub trait AccountValidate {
    /// Byte offset where `Self`'s data starts in the account buffer.
    /// - Anchor native types (`#[account]`): 8 (discriminator length)
    /// - External types (SPL `Mint` / `TokenAccount`): 0
    ///
    /// Exposed as a `const` so `Slab`'s layout constants (`HEADER_OFFSET`,
    /// `ITEMS_OFFSET`, `space_for`) can be computed at const-eval time per
    /// monomorphization.
    const DATA_OFFSET: usize;

    /// Validate the raw account data before mapping.
    /// `program_id` is available for owner checks via `Owner::owner(program_id)`.
    fn validate(view: &AccountView, data: &[u8], program_id: &Address) -> Result<(), ProgramError>;

    /// Byte offset where `Self`'s data starts in the account buffer.
    /// Default impl delegates to the `DATA_OFFSET` associated const.
    #[inline(always)]
    fn data_offset() -> usize { Self::DATA_OFFSET }
}

/// Blanket impl: every `#[account]` type (Owner + Discriminator) gets standard
/// Anchor validation — owner check via `Owner::owner(program_id)`.
impl<T: Owner + Discriminator> AccountValidate for T {
    const DATA_OFFSET: usize = 8;

    #[inline(always)]
    fn validate(view: &AccountView, data: &[u8], program_id: &Address) -> Result<(), ProgramError> {
        // Hot path: a single owner check. The "uninitialized placeholder"
        // disambiguation lives in `cold_owner_error` — placeholder accounts
        // (lamports=0, owner=system) always fail this owner check too, since
        // `T::owner(program_id)` is the user's program, never system.
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
}

/// Disambiguation for failed owner checks: uninitialized placeholder vs.
/// genuine wrong owner.
#[inline(always)]
pub(super) fn cold_owner_error(view: &AccountView) -> ProgramError {
    if view.lamports() == 0 && view.owned_by(&crate::programs::System::id()) {
        ProgramError::UninitializedAccount
    } else {
        ProgramError::IllegalOwner
    }
}

/// Error for read-only account passed to `load_mut`.
#[cfg(feature = "guardrails")]
#[inline(always)]
pub(super) fn cold_not_writable() -> ProgramError {
    ProgramError::InvalidAccountData
}

/// Defines how to create and initialize an account type via CPI.
///
/// The `Params` struct acts as a compile-time hashmap: its fields are the valid
/// init parameter keys. The macro constructs it from namespaced constraints
/// (`token::mint = mint` → `params.mint = Some(mint.account())`).
///
/// Blanket impl for `Owner + Discriminator` handles Anchor program accounts
/// (create account + write discriminator). External types (TokenAccount, Mint)
/// implement this directly with custom CPI logic.
pub trait AccountInitialize {
    type Params<'a>: Default;

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
/// zeroed by create_account, so `Slab::len` starts at 0 and items are `T::zeroed()`
/// without needing an explicit initialisation pass.
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
        // Single store on the happy path and a plain
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

// ---------------------------------------------------------------------------
// Slab<H, T>
// ---------------------------------------------------------------------------

/// Generic account type with a typed header `H` and optional trailing
/// length-prefixed array of items `T`.
///
/// ## Layout
///
/// - `[0..8]` — 8-byte discriminator (from `H::DISCRIMINATOR`)
/// - `[8..HEADER_END]` — the header `H` (read via `Deref<Target = H>`)
/// - when `T` is not a ZST:
///   - `[HEADER_END..HEADER_END+4]` — `u32 len` (current number of items)
///   - padding bytes until `ITEMS_OFFSET` is aligned to `align_of::<T>()`
///   - `[ITEMS_OFFSET..ITEMS_OFFSET + capacity * size_of::<T>()]` — raw items
/// - when `T` is a ZST (the default `Account<T> = Slab<T, ()>` case):
///   - nothing after the header; layout is byte-identical to pre-rewrite
///     `Account<T>`, no rent change, no migration.
///
/// `capacity` is derived at load time from the account's current data length,
/// so it's fully dynamic — the user picks `space` at init (or grows it via
/// `resize_to_capacity`) and this type handles the pointer math.
///
/// ## Rent responsibility
///
/// This type deliberately does **not** touch lamports during push/pop/clear
/// or during `resize_to_capacity`. Rent management is the caller's job — we
/// only expose the information they need:
///
/// - [`Slab::min_lamports`] — rent-exempt floor for the account's current size
/// - [`Slab::space_for`] — `const fn` to size a `#[account(init, space = ...)]`
/// - [`Slab::top_up`] / [`Slab::refund`] — lamport movement helpers the handler
///   composes after a resize
///
/// ## Tail-only methods
///
/// `try_push`, `pop`, `clear`, `truncate`, `swap_remove`, and `Index<usize>`
/// are compile errors when `T` is a ZST — they contain an inline `const`
/// block that panics at monomorphization time if `size_of::<T>() == 0`.
/// This means `Account<Ledger>::pop()` (which would be `Slab<Ledger, ()>::pop`)
/// fails to compile rather than silently no-opping at runtime.
///
/// ## Internals
///
/// Holds a cached typed pointer plus the pinocchio borrow guard. The guard's
/// existence is what prevents aliasing — pinocchio's refcount rejects
/// further `try_borrow*` calls while it's alive. Field access goes through
/// the cached pointer with no per-access dispatch in the common case.
///
/// The optional `guardrails` feature (default-on) adds a runtime check on
/// `Deref`/`DerefMut` that catches:
/// - Use-after-`release_borrow()` (caller forgot to `reacquire_borrow_mut`)
/// - Use-after-`close()`
/// - `DerefMut` on a read-only-loaded account (missing `#[account(mut)]`)
pub struct Slab<H, T = HeaderOnly>
where
    H: Pod + Zeroable + AccountValidate,
{
    view: AccountView,
    /// Cached pointer to the header (at `HEADER_OFFSET`). Valid while `guard`
    /// is `Some`. After `release_borrow()` or `close()`, the pointer is stale
    /// and must not be dereferenced (panics with `guardrails`).
    ///
    /// `len_ptr`, `items_ptr`, and `capacity` are NOT cached here — they're
    /// derived on demand from `header_ptr` + const offsets + `view.data_len()`.
    /// This keeps `Slab` at 3 fields (same footprint as the pre-rewrite
    /// `Account<T>`), so multi-instruction programs don't pay extra stack
    /// frame bytes at every load site.
    header_ptr: *mut H,
    guard: Option<BorrowGuard>,
    _tail: PhantomData<T>,
}

/// Marker type for the header-only form of [`Slab`]. Does **not** implement
/// `Pod`, so the tail-only `impl` block (gated on `T: Pod`) never matches —
/// calling `.push()` / `.len()` / `.as_slice()` etc. on an `Account<T>` =
/// `Slab<T, HeaderOnly>` is a compile error with "method not found" rather
/// than a runtime misbehavior.
///
/// Users shouldn't reference this type directly; use the `Account<T>`
/// alias for header-only accounts and `Slab<H, Entry>` for dynamic tails.
pub struct HeaderOnly {
    // Prevents instantiation from outside the crate.
    _private: (),
}

/// Holds the live pinocchio borrow guard for a `Slab<H, T>`. Kept around for
/// its Drop side effect (releasing the underlying borrow refcount), not its
/// data — `Slab` reads and writes through its cached pointers instead. The
/// variant distinguishes whether `DerefMut` is allowed.
#[allow(dead_code)]
enum BorrowGuard {
    Immutable(Ref<'static, [u8]>),
    Mutable(RefMut<'static, [u8]>),
}

impl<H, T> Slab<H, T>
where
    H: Pod + Zeroable + AccountValidate,
{
    /// Whether `T` is a non-zero-sized type. Folds to a const at
    /// monomorphization time.
    /// `size_of::<T>()` requires no bounds — works for any `T`, including
    /// `HeaderOnly`.
    const HAS_TAIL: bool = core::mem::size_of::<T>() > 0;

    /// Byte offset of the header. Anchor native types have an 8-byte
    /// discriminator so this is `8`; external types (SPL `Mint` /
    /// `TokenAccount`) have `0` via `H::DATA_OFFSET`.
    const HEADER_OFFSET: usize = H::DATA_OFFSET;

    /// Byte offset of the `len` field (when `HAS_TAIL`).
    const LEN_OFFSET: usize = Self::HEADER_OFFSET + core::mem::size_of::<H>();

    /// Byte offset where items start. Equal to `LEN_OFFSET` when `T` is a
    /// ZST; otherwise `LEN_OFFSET + 4`, rounded up to `align_of::<T>()`.
    const ITEMS_OFFSET: usize = {
        if core::mem::size_of::<T>() > 0 {
            let after_len = Self::LEN_OFFSET + 4;
            let a = core::mem::align_of::<T>();
            (after_len + a - 1) & !(a - 1)
        } else {
            Self::LEN_OFFSET
        }
    };

    /// Returns the account's address. Always safe regardless of borrow state.
    #[inline(always)]
    pub fn address(&self) -> &Address {
        self.view.address()
    }

    /// The underlying `AccountView` — provided for CPI callers that need the
    /// raw view.
    #[inline(always)]
    pub fn view(&self) -> &AccountView {
        &self.view
    }

    /// Release the data borrow guard so the underlying `AccountView` can be
    /// passed to CPI calls that check `is_borrowed()`. After calling this,
    /// `Deref`/`DerefMut` will panic (with `guardrails`) until
    /// `reacquire_borrow_mut()` is called.
    #[inline]
    pub fn release_borrow(&mut self) {
        self.guard = None;
    }

    /// Re-acquire an immutable borrow after a `release_borrow()` + CPI.
    pub fn reacquire_borrow(&mut self) -> core::result::Result<(), ProgramError> {
        let data_ref = self.view.try_borrow()?;
        // SAFETY: AccountView's raw pointer outlives this instruction.
        let guard: Ref<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        self.header_ptr = unsafe {
            (guard.as_ptr() as *mut u8).add(Self::HEADER_OFFSET)
        } as *mut H;
        self.guard = Some(BorrowGuard::Immutable(guard));
        Ok(())
    }

    /// Re-acquire a mutable borrow after a `release_borrow()` + CPI.
    pub fn reacquire_borrow_mut(&mut self) -> core::result::Result<(), ProgramError> {
        let mut view_mut = self.view;
        let data_ref = view_mut.try_borrow_mut()?;
        let mut guard: RefMut<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        self.header_ptr = unsafe {
            guard.as_mut_ptr().add(Self::HEADER_OFFSET)
        } as *mut H;
        self.guard = Some(BorrowGuard::Mutable(guard));
        Ok(())
    }

    /// Validate `len <= capacity` for the tail region before we do the
    /// lifetime transmute. Works on `&[u8]` directly — no unsafe, no
    /// alignment concerns (uses `u32::from_le_bytes` on a stack copy).
    #[inline(always)]
    fn validate_tail(data: &[u8]) -> Result<(), ProgramError> {
        if !Self::HAS_TAIL {
            return Ok(());
        }
        let data_len = data.len();
        let capacity = (data_len - Self::ITEMS_OFFSET) / core::mem::size_of::<T>();
        let mut len_bytes = [0u8; 4];
        len_bytes.copy_from_slice(&data[Self::LEN_OFFSET..Self::LEN_OFFSET + 4]);
        let len = u32::from_le_bytes(len_bytes) as usize;
        if len > capacity {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    #[inline(always)]
    fn from_ref(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        let data_ref = view.try_borrow()?;
        H::validate(&view, &data_ref, program_id)?;
        if data_ref.len() < Self::ITEMS_OFFSET {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Self::validate_tail(&data_ref)?;
        // SAFETY: extend lifetime — the underlying data outlives any local
        // scope within the instruction, and the Ref guard prevents aliasing.
        let guard: Ref<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        let header_ptr = unsafe {
            (guard.as_ptr() as *mut u8).add(Self::HEADER_OFFSET)
        } as *mut H;
        Ok(Self {
            view,
            header_ptr,
            guard: Some(BorrowGuard::Immutable(guard)),
            _tail: PhantomData,
        })
    }

    /// Low-level constructor: acquire a mutable borrow, set up `header_ptr`,
    /// return a `Slab` with no validation. Shared by `load_mut_after_init`
    /// (which calls it directly) and `load_mut` (which calls it via
    /// `load_mut_after_init` then validates on top).
    ///
    /// Under `guardrails`, includes a minimum-length check so the
    /// cached `header_ptr` points at bytes that actually exist in the
    /// account data region.
    #[inline(always)]
    fn build_mutable(view: AccountView) -> Result<Self, ProgramError> {
        let mut view_mut = view;
        let data_ref = view_mut.try_borrow_mut()?;
        #[cfg(feature = "guardrails")]
        if data_ref.len() < Self::ITEMS_OFFSET {
            return Err(ProgramError::AccountDataTooSmall);
        }
        // SAFETY: same lifetime-transmute pattern as `from_ref`. The
        // underlying data buffer lives for the whole instruction and the
        // guard prevents aliasing.
        let mut guard: RefMut<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        // Derive header_ptr through DerefMut (as_mut_ptr) to preserve write
        // provenance. Using as_ptr() routes through Deref → *const, losing
        // write provenance under Stacked Borrows / Tree Borrows.
        let header_ptr = unsafe {
            guard.as_mut_ptr().add(Self::HEADER_OFFSET)
        } as *mut H;
        Ok(Self {
            view,
            header_ptr,
            guard: Some(BorrowGuard::Mutable(guard)),
            _tail: PhantomData,
        })
    }

    // -----------------------------------------------------------------------
    // Rent helpers — work for both header-only and tail forms.
    // -----------------------------------------------------------------------

    /// Rent-exempt lamport minimum for the account's **current** data length.
    ///
    /// Minimum lamports for rent exemption at the current account size.
    /// Uses runtime sysvar by default; `const-rent` feature uses baked-in rate.
    #[inline]
    pub fn min_lamports(&self) -> Result<u64, ProgramError> {
        crate::cpi::rent_exempt_lamports(self.view.data_len())
    }

    /// Current size of the account's data region in bytes.
    #[inline(always)]
    pub fn current_space(&self) -> usize {
        self.view.data_len()
    }

    /// Pay the rent shortfall from `payer`. No-op if the account already
    /// holds at least `min_lamports()`.
    ///
    /// Uses a `system::Transfer` CPI; `payer` must be a signer on the outer
    /// transaction (pinocchio enforces signerness at CPI time).
    pub fn top_up(&mut self, payer: &AccountView) -> Result<(), ProgramError> {
        let required = self.min_lamports()?;
        let current = self.view.lamports();
        if current >= required {
            return Ok(());
        }
        let deficit = required - current;
        pinocchio_system::instructions::Transfer {
            from: payer,
            to: &self.view,
            lamports: deficit,
        }
        .invoke()
    }

    /// Move excess lamports (current - min_lamports) from the account to
    /// `recipient`. No-op if the account is already at the rent floor.
    ///
    /// Direct lamport arithmetic, no CPI — safe because the account is
    /// program-owned (which is always the case when you hold a `Slab`).
    pub fn refund(&mut self, recipient: &mut AccountView) -> Result<(), ProgramError> {
        let required = self.min_lamports()?;
        let current = self.view.lamports();
        if current <= required {
            return Ok(());
        }
        let excess = current - required;
        let new_recipient = recipient
            .lamports()
            .checked_add(excess)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        recipient.set_lamports(new_recipient);
        let mut self_view = self.view;
        self_view.set_lamports(required);
        Ok(())
    }
}

// ===========================================================================
// Tail-only impl block
//
// The `T: Pod` bound makes every method in this block *invisible* for
// `Slab<H, HeaderOnly>` = `Account<H>`, because `HeaderOnly` doesn't
// implement `Pod`. Calling `.len()` / `.push()` / `.as_slice()` on an
// `Account<Counter>` becomes a plain "method not found" compile error.
// ===========================================================================

impl<H, T> Slab<H, T>
where
    H: Pod + Zeroable + AccountValidate,
    T: Pod,
{
    // -----------------------------------------------------------------------
    // Safe byte-slice accessors — bounds checks + bytemuck alignment checks
    // trade a small cost for zero unsafe in the tail-mutation path.
    //
    // `Deref<Target = H>` still uses the cached `header_ptr` for zero-cost
    // field access — the hot path for `ctx.accounts.ledger.authority` is
    // unchanged.
    // -----------------------------------------------------------------------

    /// The account data bytes via the currently-held borrow guard. Panics if
    /// `release_borrow()` or `close()` dropped the guard.
    #[inline(always)]
    fn guard_bytes(&self) -> &[u8] {
        match &self.guard {
            Some(BorrowGuard::Immutable(r)) => r,
            Some(BorrowGuard::Mutable(r)) => r,
            None => panic!(
                "Slab<H, T> accessed after release_borrow() or close(). \
                 Call reacquire_borrow_mut() before touching the tail."
            ),
        }
    }

    /// Mutable variant. Panics if the slab was loaded read-only, or if
    /// the guard was dropped.
    #[inline(always)]
    fn guard_bytes_mut(&mut self) -> &mut [u8] {
        match &mut self.guard {
            Some(BorrowGuard::Mutable(r)) => r,
            Some(BorrowGuard::Immutable(_)) => panic!(
                "Slab<H, T> mutated through a read-only guard. \
                 Add #[account(mut)] to your accounts struct."
            ),
            None => panic!(
                "Slab<H, T> mutated after release_borrow() or close(). \
                 Call reacquire_borrow_mut() before touching the tail."
            ),
        }
    }

    /// Read the `len` field without requiring `LEN_OFFSET` alignment —
    /// `from_le_bytes` operates on a copy, so misaligned layouts are fine.
    #[inline(always)]
    fn read_len(&self) -> u32 {
        let bytes = self.guard_bytes();
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&bytes[Self::LEN_OFFSET..Self::LEN_OFFSET + 4]);
        u32::from_le_bytes(buf)
    }

    /// Write the `len` field. Same alignment-free pattern as `read_len`.
    #[inline(always)]
    fn write_len(&mut self, new_len: u32) {
        let bytes = self.guard_bytes_mut();
        bytes[Self::LEN_OFFSET..Self::LEN_OFFSET + 4]
            .copy_from_slice(&new_len.to_le_bytes());
    }

    /// Total account data size required to hold the header plus `capacity`
    /// items. `const fn`, so callers can put it directly into
    /// `#[account(init, space = Slab::<Ledger, Entry>::space_for(64), ...)]`.
    #[inline(always)]
    pub const fn space_for(capacity: usize) -> usize {
        Self::ITEMS_OFFSET + capacity * core::mem::size_of::<T>()
    }

    /// Current number of items in the tail region.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.read_len() as usize
    }

    /// How many items the account's tail region can currently hold without
    /// growing. Derived on demand from `view.data_len()`.
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        (self.view.data_len() - Self::ITEMS_OFFSET) / core::mem::size_of::<T>()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    /// View the tail region as an immutable slice.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        let len = self.len();
        let bytes = self.guard_bytes();
        // `ITEMS_OFFSET` is const-computed to be `align_of::<T>()`-aligned,
        // and Pod requires `size_of` is a multiple of `align_of`, so every
        // per-item offset is aligned. bytemuck will verify this at runtime.
        let items_bytes = &bytes[Self::ITEMS_OFFSET..Self::ITEMS_OFFSET + len * core::mem::size_of::<T>()];
        bytemuck::cast_slice(items_bytes)
    }

    /// View the tail region as a mutable slice.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let len = self.len();
        let bytes = self.guard_bytes_mut();
        let items_bytes = &mut bytes[Self::ITEMS_OFFSET..Self::ITEMS_OFFSET + len * core::mem::size_of::<T>()];
        bytemuck::cast_slice_mut(items_bytes)
    }

    #[inline(always)]
    pub fn iter(&self) -> core::slice::Iter<'_, T> {
        self.as_slice().iter()
    }

    #[inline(always)]
    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, T> {
        self.as_mut_slice().iter_mut()
    }

    #[inline(always)]
    pub fn first(&self) -> Option<&T> {
        self.as_slice().first()
    }

    #[inline(always)]
    pub fn last(&self) -> Option<&T> {
        self.as_slice().last()
    }

    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.as_slice().get(index)
    }

    #[inline(always)]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.as_mut_slice().get_mut(index)
    }

    // -----------------------------------------------------------------------
    // Tail-region mutations — all safe, go through `guard_bytes_mut()`.
    // -----------------------------------------------------------------------

    /// Append `value` to the tail region.
    ///
    /// Returns `Err(AccountDataTooSmall)` when `len == capacity`. The caller
    /// is responsible for growing the account via `resize_to_capacity`
    /// beforehand.
    pub fn try_push(&mut self, value: T) -> Result<(), ProgramError> {
        let len = self.len();
        if len >= self.capacity() {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let item_offset = Self::ITEMS_OFFSET + len * core::mem::size_of::<T>();
        {
            let bytes = self.guard_bytes_mut();
            let slot = &mut bytes[item_offset..item_offset + core::mem::size_of::<T>()];
            *bytemuck::from_bytes_mut::<T>(slot) = value;
        }
        self.write_len((len + 1) as u32);
        Ok(())
    }

    /// Remove and return the last item, or `None` if empty.
    pub fn pop(&mut self) -> Option<T> {
        let len = self.len();
        if len == 0 {
            return None;
        }
        let new_len = len - 1;
        let item_offset = Self::ITEMS_OFFSET + new_len * core::mem::size_of::<T>();
        let value = {
            let bytes = self.guard_bytes();
            let slot = &bytes[item_offset..item_offset + core::mem::size_of::<T>()];
            *bytemuck::from_bytes::<T>(slot)
        };
        self.write_len(new_len as u32);
        Some(value)
    }

    /// Truncate the tail to `new_len`. No-op if `new_len >= len`.
    pub fn truncate(&mut self, new_len: usize) {
        if new_len < self.len() {
            self.write_len(new_len as u32);
        }
    }

    /// Clear the tail region (set `len` to 0). Does not zero item memory.
    pub fn clear(&mut self) {
        self.write_len(0);
    }

    /// Swap the item at `index` with the last, then pop. `O(1)` remove.
    ///
    /// # Panics
    ///
    /// Panics if `index >= len`, matching `Vec::swap_remove`.
    pub fn swap_remove(&mut self, index: usize) -> T {
        let len = self.len();
        assert!(index < len, "swap_remove index out of bounds");
        let new_len = len - 1;
        // Go through the typed slice — `as_mut_slice()` returns a bounds-
        // checked `&mut [T]` of length `len`, and `swap` + read are safe.
        let removed = {
            let items = self.as_mut_slice();
            let value = items[index];
            if index != new_len {
                items[index] = items[new_len];
            }
            value
        };
        self.write_len(new_len as u32);
        removed
    }

    /// Resize the account's data region to hold `new_capacity` items without
    /// touching lamports. After calling this, compose with `top_up` (grow)
    /// or `refund` (shrink) to get back to the rent floor.
    ///
    /// Drops and re-acquires the borrow guard across the `resize` call.
    /// The guard's `RefMut<[u8]>` has a frozen slice length captured at
    /// borrow time, so holding it across `resize` would leave `self.guard`
    /// pointing at a stale-length view of the data region. On grow, the
    /// stale slice would miss the newly-allocated tail; on shrink, it
    /// would extend past the new data end into pinocchio's padding. Both
    /// cases manifest as bounds-check panics on the next tail op. The
    /// drop-and-reborrow dance avoids that footgun.
    ///
    /// `header_ptr` is re-derived from the fresh guard — on SBF it points
    /// at the same address (the data region is stable — pinocchio
    /// pre-allocates `MAX_PERMITTED_DATA_INCREASE` bytes of padding after
    /// each account), but re-deriving is cheap and future-proofs against
    /// any runtime that *does* relocate the buffer.
    #[cfg(feature = "account-resize")]
    pub fn resize_to_capacity(&mut self, new_capacity: usize) -> Result<(), ProgramError> {
        use pinocchio::Resize;

        let new_space = Self::space_for(new_capacity);
        // Drop the stale-length guard before calling resize. pinocchio's
        // refcount won't let us have a live RefMut and also mutate data_len
        // cleanly, so we release and re-acquire.
        self.guard = None;
        let mut view_mut = self.view;
        view_mut.resize(new_space)?;
        // Re-acquire with the new length and re-derive header_ptr.
        let data_ref = view_mut.try_borrow_mut()?;
        // SAFETY: same lifetime-transmute pattern as `build_mutable`.
        let mut guard: RefMut<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        self.header_ptr = unsafe {
            guard.as_mut_ptr().add(Self::HEADER_OFFSET)
        } as *mut H;
        self.guard = Some(BorrowGuard::Mutable(guard));
        // Clamp len down if we shrunk below the current item count.
        let new_cap = self.capacity();
        if self.len() > new_cap {
            self.write_len(new_cap as u32);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// AnchorAccount / Deref / Index / AsRef impls
// ---------------------------------------------------------------------------

impl<H, T> AnchorAccount for Slab<H, T>
where
    H: Pod + Zeroable + AccountValidate,
{
    type Data = H;
    const MIN_DATA_LEN: usize = 8;

    #[inline(always)]
    fn load(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        Self::from_ref(view, program_id)
    }

    #[inline(always)]
    fn load_mut(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        // Reuses the post-init primitive for construction, then layers
        // full validation on top. `load_mut_after_init` already handles
        // the writable check and the mutable borrow; we validate through
        // the guard bytes it sets up, avoiding a second `try_borrow` pair.
        let slab = Self::load_mut_after_init(view, program_id)?;
        // SAFETY: `load_mut_after_init` always returns with a `Mutable`
        // guard set on success.
        let data: &[u8] = match &slab.guard {
            Some(BorrowGuard::Mutable(r)) => r,
            _ => unreachable!("load_mut_after_init returns Mutable guard"),
        };
        H::validate(&slab.view, data, program_id)?;
        if data.len() < Self::ITEMS_OFFSET {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Self::validate_tail(data)?;
        Ok(slab)
    }

    /// Fast-path `load_mut` for the post-`create_and_initialize` case. Skips
    /// `H::validate` (owner / disc / min-length all tautologies: the
    /// system program set the owner, we just wrote the disc, and
    /// `create_account` allocated exactly `ITEMS_OFFSET + 0 * size_of::<T>()`
    /// bytes). Also skips `validate_tail` because `len == 0` is guaranteed
    /// by the zero-init semantics of `create_account`.
    ///
    /// Under `guardrails`, `build_mutable` still does a length check so the
    /// cached `header_ptr` is valid. Under `guardrails = off`, drops it too.
    #[inline(always)]
    fn load_mut_after_init(
        view: AccountView,
        _program_id: &Address,
    ) -> Result<Self, ProgramError> {
        // Guardrail: catches "forgot `#[account(mut)]`" early with a clear
        // error. Under `default-features = false` the Solana runtime still
        // rejects the tx when we try to write, just with a less specific
        // message. Compiled out without guardrails.
        #[cfg(feature = "guardrails")]
        if !view.is_writable() {
            return Err(cold_not_writable());
        }
        Self::build_mutable(view)
    }

    #[inline(always)]
    fn account(&self) -> &AccountView {
        &self.view
    }

    fn close(&mut self, mut destination: AccountView) -> pinocchio::ProgramResult {
        // Drop the borrow guard before mutating the underlying account
        // state, so any nested helpers can re-borrow cleanly.
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

impl<H, T> Deref for Slab<H, T>
where
    H: Pod + Zeroable + AccountValidate,
{
    type Target = H;

    #[inline(always)]
    fn deref(&self) -> &H {
        #[cfg(feature = "guardrails")]
        if self.guard.is_none() {
            panic!(
                "Slab<H, T> dereferenced after release_borrow() or close(). \
                 Call reacquire_borrow_mut() before accessing fields again."
            );
        }
        // SAFETY: while `guard` is `Some`, pinocchio's refcount holds the
        // borrow open and prevents aliasing.
        unsafe { &*self.header_ptr }
    }
}

impl<H, T> DerefMut for Slab<H, T>
where
    H: Pod + Zeroable + AccountValidate,
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut H {
        // Always checked (not guardrails-gated): creating `&mut H` from a
        // pointer derived from a `Ref` (shared borrow) is UB. The guard
        // check is the only thing preventing it, so it must run even in
        // release builds.
        match &self.guard {
            None => panic!(
                "Slab<H, T> mutably dereferenced after release_borrow() or close(). \
                 Call reacquire_borrow_mut() before accessing fields again."
            ),
            Some(BorrowGuard::Immutable(_)) => panic!(
                "Slab<H, T> mutably dereferenced but loaded read-only. \
                 Add #[account(mut)] to your accounts struct."
            ),
            Some(BorrowGuard::Mutable(_)) => {}
        }
        // SAFETY: under a Mutable guard, the pointer was derived from a
        // RefMut (exclusive borrow) with write provenance. No other live
        // borrow exists; we hold `&mut self`.
        unsafe { &mut *self.header_ptr }
    }
}

// `T: Pod` bound matches the tail-only impl block — only reachable for
// `Slab<H, T>` where `T` is a real pod type, not `HeaderOnly`.
impl<H, T> Index<usize> for Slab<H, T>
where
    H: Pod + Zeroable + AccountValidate,
    T: Pod,
{
    type Output = T;

    #[inline(always)]
    fn index(&self, index: usize) -> &T {
        &self.as_slice()[index]
    }
}

impl<H, T> IndexMut<usize> for Slab<H, T>
where
    H: Pod + Zeroable + AccountValidate,
    T: Pod,
{
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut T {
        &mut self.as_mut_slice()[index]
    }
}

impl<H, T> AsRef<AccountView> for Slab<H, T>
where
    H: Pod + Zeroable + AccountValidate,
{
    #[inline(always)]
    fn as_ref(&self) -> &AccountView {
        &self.view
    }
}

impl<H, T> AsRef<Address> for Slab<H, T>
where
    H: Pod + Zeroable + AccountValidate,
{
    #[inline(always)]
    fn as_ref(&self) -> &Address {
        self.view.address()
    }
}
