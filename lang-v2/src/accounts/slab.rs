use {
    super::slab_hooks::{SlabInit, SlabSchema},
    crate::{AccountInitialize, AnchorAccount, Discriminator, Id},
    bytemuck::{Pod, Zeroable},
    core::{
        marker::PhantomData,
        ops::{Deref, DerefMut, Index, IndexMut},
    },
    pinocchio::{
        account::AccountView,
        address::Address,
    },
    solana_program_error::ProgramError,
};

// SlabSchema / SlabInit (bytes-level hooks Slab invokes on `H`) live in
// `accounts::slab_hooks`. The forwards below tie them + the
// wrapper-level `AccountInitialize` together.

/// Disambiguation for failed owner checks: uninitialized placeholder vs.
/// genuine wrong owner. Used by `SlabSchema`'s blanket impl (via `super::`).
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

/// `Account<H>` / `BorshAccount<H>` get `AccountInitialize` for free by
/// running `H::SlabInit::create_and_initialize(...)` and then loading.
impl<H, T> AccountInitialize for Slab<H, T>
where
    H: SlabInit + Pod + Zeroable + SlabSchema,
    Self: AnchorAccount,
{
    type Params<'a> = H::Params<'a>;

    #[inline(always)]
    fn create_and_initialize<'a>(
        payer: &AccountView,
        account: &AccountView,
        space: usize,
        program_id: &Address,
        params: &Self::Params<'a>,
        signer_seeds: Option<&[&[u8]]>,
    ) -> Result<Self, ProgramError> {
        H::create_and_initialize(payer, account, space, program_id, params, signer_seeds)?;
        // SAFETY: `create_and_initialize` just created this account; no other
        // mutable reference to its data can exist yet.
        unsafe { <Self as AnchorAccount>::load_mut_after_init(*account, program_id) }
    }
}

/// Forward `Discriminator` from a `Slab<H, _>` to its header type. Lets the
/// `#[account(zeroed)]` derive codegen look up the disc via the field type
/// directly (e.g. `<Account<Counter> as Discriminator>::DISCRIMINATOR`)
/// instead of extracting an inner type by string-matching on "Account".
impl<H, T> Discriminator for Slab<H, T>
where
    H: Discriminator + Pod + Zeroable + SlabSchema,
{
    const DISCRIMINATOR: &'static [u8] = H::DISCRIMINATOR;
}

// `INIT_SPACE = 8 + size_of::<H>()` — no `H: Space` bound needed since
// `H: Pod` guarantees a fully-defined layout. Gives the full on-wire size
// for header-only accounts; item-carrying Slabs should specify `space`
// explicitly.
impl<H, T> crate::Space for Slab<H, T>
where
    H: Pod + Zeroable + SlabSchema,
{
    const INIT_SPACE: usize = 8 + core::mem::size_of::<H>();
}

// ---------------------------------------------------------------------------
// Slab<H, T>
// ---------------------------------------------------------------------------

/// Generic account type with a typed header `H` and optional trailing
/// length-prefixed array of items `T`.
///
/// ## Layout
///
/// `[disc:8][H]` — when `T` is a ZST (`Account<T> = Slab<T, HeaderOnly>`).
/// `[disc:8][H][len:u32][pad][items...]` — when `T: Pod` (non-ZST).
/// Capacity is derived from `view.data_len()` at load time.
///
/// ## Rent responsibility
///
/// Push/pop/clear/`resize_to_capacity` do **not** touch lamports. Use
/// [`min_lamports`](Slab::min_lamports), [`top_up`](Slab::top_up),
/// [`refund`](Slab::refund), and [`space_for`](Slab::space_for).
///
/// ## Tail-only methods
///
/// `try_push`, `pop`, `clear`, `truncate`, `swap_remove`, `Index<usize>`
/// require `T: Pod`; `HeaderOnly` doesn't impl `Pod`, so these are
/// compile errors on `Account<T>`.
///
/// ## Internals
///
/// Caches a typed pointer to `H` (valid for the instruction lifetime).
/// `is_mutable` gates `DerefMut` to catch missing `#[account(mut)]`.
pub struct Slab<H, T = HeaderOnly>
where
    H: Pod + Zeroable + SlabSchema,
{
    view: AccountView,
    /// Cached pointer to the header (at `HEADER_OFFSET`). Valid for the
    /// entire instruction lifetime (Solana runtime guarantee).
    ///
    /// `len_ptr`, `items_ptr`, and `capacity` are NOT cached here — they're
    /// derived on demand from `header_ptr` + const offsets + `view.data_len()`.
    /// This keeps `Slab` at 3 fields (same footprint as the pre-rewrite
    /// `Account<T>`), so multi-instruction programs don't pay extra stack
    /// frame bytes at every load site.
    header_ptr: *mut H,
    /// Whether this slab was loaded via `load_mut`. Guards `DerefMut` to catch
    /// missing `#[account(mut)]` at the point of access rather than silently
    /// producing UB through a const-provenance pointer.
    is_mutable: bool,
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

impl<H, T> Slab<H, T>
where
    H: Pod + Zeroable + SlabSchema,
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
        // SAFETY: AccountView's data pointer is valid for the instruction lifetime
        // (Solana runtime guarantee). Duplicate mutable accounts are rejected at
        // deserialization, so no aliasing can occur.
        let data = unsafe { view.borrow_unchecked() };
        H::validate(&view, data, program_id)?;
        if data.len() < Self::ITEMS_OFFSET {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Self::validate_tail(data)?;
        let header_ptr = unsafe { view.data_ptr().add(Self::HEADER_OFFSET) } as *mut H;
        // Leave `borrow_state = NOT_BORROWED` untouched. Duplicate-mutable
        // detection runs at deserialization time in the derive macro, and
        // leaving the byte clear means downstream pinocchio CPIs can call
        // `try_borrow*()` against this account's view without an explicit
        // release step on the Slab side.
        Ok(Self {
            view,
            header_ptr,
            is_mutable: false,
            _tail: PhantomData,
        })
    }

    /// Low-level constructor: set up `header_ptr` with write provenance,
    /// no validation. Under `guardrails`, includes a minimum-length check.
    #[inline(always)]
    fn build_mutable(view: AccountView) -> Result<Self, ProgramError> {
        // SAFETY: AccountView's data pointer is valid for the instruction lifetime.
        // Duplicate mutable accounts are rejected at deserialization.
        #[cfg(feature = "guardrails")]
        {
            let data = unsafe { view.borrow_unchecked() };
            if data.len() < Self::ITEMS_OFFSET {
                return Err(ProgramError::AccountDataTooSmall);
            }
        }
        // Derive header_ptr through data_mut_ptr to preserve write provenance.
        // Using data_ptr → *const would lose it under Stacked Borrows / Tree Borrows.
        let mut view_mut = view;
        let header_ptr = unsafe { view_mut.data_mut_ptr().add(Self::HEADER_OFFSET) } as *mut H;
        // Leave `borrow_state = NOT_BORROWED` untouched — same rationale as
        // `from_ref`. Slab accesses data via `borrow_unchecked*()` anyway
        // (the cached `header_ptr`), so the marker byte would not gate our
        // own reads — only downstream pinocchio CPIs.
        Ok(Self {
            view,
            header_ptr,
            is_mutable: true,
            _tail: PhantomData,
        })
    }

    // -----------------------------------------------------------------------
    // Rent helpers — work for both header-only and tail forms.
    // -----------------------------------------------------------------------

    /// Rent-exempt lamport minimum for the account's current data length.
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
// Tail-only impl block — `T: Pod` bound excludes `HeaderOnly`, so these
// methods are "method not found" compile errors on `Account<H>`.
// ===========================================================================

impl<H, T> Slab<H, T>
where
    H: Pod + Zeroable + SlabSchema,
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

    /// The account data bytes. Always valid for the instruction lifetime.
    #[inline(always)]
    fn guard_bytes(&self) -> &[u8] {
        // SAFETY: AccountView data is valid for the instruction lifetime.
        // Duplicate mutable accounts are rejected at deserialization.
        unsafe { self.view.borrow_unchecked() }
    }

    /// Mutable account data bytes. Panics if the slab was loaded read-only.
    #[inline(always)]
    fn guard_bytes_mut(&mut self) -> &mut [u8] {
        if !self.is_mutable {
            panic!(
                "Slab<H, T> mutated through a read-only load. Add #[account(mut)] to your \
                 accounts struct."
            );
        }
        // SAFETY: is_mutable guarantees this was loaded via load_mut.
        // AccountView data is valid for the instruction lifetime.
        unsafe { self.view.borrow_unchecked_mut() }
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
        bytes[Self::LEN_OFFSET..Self::LEN_OFFSET + 4].copy_from_slice(&new_len.to_le_bytes());
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

    /// How many items the account's tail region can currently hold.
    /// Returns 0 if `data_len < ITEMS_OFFSET` (guards against post-resize
    /// underflow when an external `realloc_account` has shrunk the
    /// account below the Slab's structural minimum).
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.view
            .data_len()
            .saturating_sub(Self::ITEMS_OFFSET)
            / core::mem::size_of::<T>()
    }

    /// Live `len` clamped to current `capacity`. The stored `len` may
    /// exceed `capacity` if an external `realloc_account` shrank the
    /// account after this `Slab` was constructed; mutation paths must
    /// use this value (not the raw `len`) when computing item offsets
    /// or indexing the tail region.
    #[inline(always)]
    fn effective_len(&self) -> usize {
        self.len().min(self.capacity())
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    /// View the tail region as an immutable slice. Uses `effective_len` so
    /// a post-load external resize (which would leave raw `len > capacity`)
    /// cannot cause an OOB slice read.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        let len = self.effective_len();
        let bytes = self.guard_bytes();
        // `ITEMS_OFFSET` is const-computed to be `align_of::<T>()`-aligned,
        // and Pod requires `size_of` is a multiple of `align_of`, so every
        // per-item offset is aligned. bytemuck will verify this at runtime.
        let items_bytes =
            &bytes[Self::ITEMS_OFFSET..Self::ITEMS_OFFSET + len * core::mem::size_of::<T>()];
        bytemuck::cast_slice(items_bytes)
    }

    /// View the tail region as a mutable slice. Uses `effective_len`;
    /// same post-resize guard as `as_slice`.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let len = self.effective_len();
        let bytes = self.guard_bytes_mut();
        let items_bytes =
            &mut bytes[Self::ITEMS_OFFSET..Self::ITEMS_OFFSET + len * core::mem::size_of::<T>()];
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

    /// Remove and return the last item, or `None` if empty. Uses
    /// `effective_len` so a post-shrink Slab (raw len > capacity) doesn't
    /// read past the live data buffer; the write-back also clamps the
    /// stored len to a value `<= capacity`.
    pub fn pop(&mut self) -> Option<T> {
        let len = self.effective_len();
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

    /// Truncate the tail to `new_len`. Uses `effective_len` so a
    /// post-shrink Slab is brought back to a consistent state: the stored
    /// len ends up at `min(new_len, effective_len)`.
    pub fn truncate(&mut self, new_len: usize) {
        let target = new_len.min(self.effective_len());
        if target != self.len() {
            self.write_len(target as u32);
        }
    }

    /// Clear the tail region (set `len` to 0). Does not zero item memory.
    pub fn clear(&mut self) {
        self.write_len(0);
    }

    /// Swap the item at `index` with the last, then pop. `O(1)` remove.
    /// Uses `effective_len` so a post-shrink Slab can't index past the
    /// live data buffer.
    ///
    /// # Panics
    ///
    /// Panics if `index >= effective_len()`, matching `Vec::swap_remove`.
    pub fn swap_remove(&mut self, index: usize) -> T {
        let len = self.effective_len();
        assert!(index < len, "swap_remove index out of bounds");
        let new_len = len - 1;
        // `as_mut_slice()` returns a bounds-checked `&mut [T]` of length
        // `effective_len`, so `index` and `new_len` are both in-bounds.
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
    /// touching lamports. Compose with `top_up` / `refund` afterward to
    /// settle rent. Re-derives `header_ptr` after the resize; `guard_bytes*`
    /// pick up the new size from `view.data_len()` automatically.
    #[cfg(feature = "account-resize")]
    pub fn resize_to_capacity(&mut self, new_capacity: usize) -> Result<(), ProgramError> {
        use pinocchio::Resize;

        let new_space = Self::space_for(new_capacity);
        let mut view_mut = self.view;
        // SAFETY: Slab owns exclusive access to the data (enforced by the
        // borrow flag set in build_mutable). Use resize_unchecked to bypass
        // pinocchio's check_borrow_mut() which would see our flag and fail.
        unsafe { view_mut.resize_unchecked(new_space)? };
        // Re-derive header_ptr with write provenance in case the runtime
        // relocated the buffer.
        self.header_ptr = unsafe { view_mut.data_mut_ptr().add(Self::HEADER_OFFSET) } as *mut H;
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
    H: Pod + Zeroable + SlabSchema,
{
    type Data = H;
    const MIN_DATA_LEN: usize = 8;

    #[inline(always)]
    fn load(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        Self::from_ref(view, program_id)
    }

    /// # Safety
    ///
    /// See [`AnchorAccount::load_mut`] — caller must ensure no other live
    /// `&mut` to the same account data exists.
    #[inline(always)]
    unsafe fn load_mut(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        // Reuses the post-init primitive for construction, then layers full
        // validation on top.
        let slab = Self::load_mut_after_init(view, program_id)?;
        // SAFETY: build_mutable succeeded, so the data pointer is valid.
        let data: &[u8] = unsafe { slab.view.borrow_unchecked() };
        H::validate(&slab.view, data, program_id)?;
        if data.len() < Self::ITEMS_OFFSET {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Self::validate_tail(data)?;
        Ok(slab)
    }

    /// Fast-path `load_mut` after `create_and_initialize`. Skips
    /// `H::validate` and `validate_tail` (all tautologies post-init).
    ///
    /// # Safety
    ///
    /// See [`AnchorAccount::load_mut`] — no other live `&mut` to the
    /// same account data.
    #[inline(always)]
    unsafe fn load_mut_after_init(
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
        let mut self_view = self.view;
        let dest_lamports = destination
            .lamports()
            .checked_add(self_view.lamports())
            .ok_or(ProgramError::ArithmeticOverflow)?;
        destination.set_lamports(dest_lamports);
        self_view.set_lamports(0);
        // SAFETY: Slab owns exclusive access (borrow flag is set). Use
        // close_unchecked to bypass pinocchio's is_borrowed() check.
        unsafe { self_view.close_unchecked() };
        Ok(())
    }
}

impl<H, T> Deref for Slab<H, T>
where
    H: Pod + Zeroable + SlabSchema,
{
    type Target = H;

    #[inline(always)]
    fn deref(&self) -> &H {
        // SAFETY: header_ptr is valid for the instruction lifetime (Solana
        // runtime guarantee). Duplicate mutable accounts are rejected at
        // deserialization, so no aliasing can occur.
        unsafe { &*self.header_ptr }
    }
}

impl<H, T> DerefMut for Slab<H, T>
where
    H: Pod + Zeroable + SlabSchema,
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut H {
        // Always checked (not guardrails-gated): creating `&mut H` from a
        // const-provenance pointer is UB, so this must run even in release.
        if !self.is_mutable {
            panic!(
                "Slab<H, T> mutably dereferenced but loaded read-only. Add #[account(mut)] to \
                 your accounts struct."
            );
        }
        // SAFETY: is_mutable guarantees header_ptr was derived via data_mut_ptr
        // (write provenance). No other live mutable borrow exists; we hold &mut self.
        unsafe { &mut *self.header_ptr }
    }
}

// `T: Pod` bound matches the tail-only impl block — only reachable for
// `Slab<H, T>` where `T` is a real pod type, not `HeaderOnly`.
impl<H, T> Index<usize> for Slab<H, T>
where
    H: Pod + Zeroable + SlabSchema,
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
    H: Pod + Zeroable + SlabSchema,
    T: Pod,
{
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut T {
        &mut self.as_mut_slice()[index]
    }
}

impl<H, T> AsRef<AccountView> for Slab<H, T>
where
    H: Pod + Zeroable + SlabSchema,
{
    #[inline(always)]
    fn as_ref(&self) -> &AccountView {
        &self.view
    }
}

impl<H, T> AsRef<Address> for Slab<H, T>
where
    H: Pod + Zeroable + SlabSchema,
{
    #[inline(always)]
    fn as_ref(&self) -> &Address {
        self.view.address()
    }
}

#[cfg(feature = "idl-build")]
impl<H, T> crate::IdlAccountType for Slab<H, T>
where
    H: Pod + Zeroable + SlabSchema + crate::IdlAccountType,
{
    const __IDL_TYPE: Option<&'static str> = H::__IDL_TYPE;
    fn __register_idl_deps(types: &mut ::alloc::vec::Vec<&'static str>) {
        H::__register_idl_deps(types);
    }
}

