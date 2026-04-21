use {
    crate::{AccountInitialize, AnchorAccount, Discriminator, Owner},
    borsh::{BorshDeserialize, BorshSerialize},
    core::ops::{Deref, DerefMut},
    pinocchio::{
        account::{AccountView, Ref, RefMut},
        address::Address,
    },
    solana_program_error::ProgramError,
};

/// Discriminator length in bytes. All `#[account]` types use an 8-byte
/// discriminator; borsh accounts prefix their data with it.
const DISC_LEN: usize = 8;

/// Borsh-serialized account type.
///
/// Validates owner, checks discriminator, deserializes via borsh. Holds a
/// pinocchio borrow guard (`Ref` for `load`, `RefMut` for `load_mut`);
/// `exit()` serializes through the held `RefMut`.
///
/// ## `#[account(owner = X @ MyErr)]` does NOT surface `MyErr`
///
/// Owner/discriminator validation runs inside `load`/`load_mut`, before
/// derive-level constraints. A mismatch is `ProgramError::IllegalOwner`,
/// not the user's `@ MyErr`. For a custom error, use `UncheckedAccount`
/// with derive-level `owner = X @ MyErr` (you lose the built-in disc/borsh
/// checks).
pub struct BorshAccount<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> {
    view: AccountView,
    data: T,
    borrow: BorshBorrow,
    /// Owner snapshotted at load time. `reacquire_borrow_mut` re-checks
    /// `view.owned_by(&expected_owner)` because a released-window CPI
    /// could transfer the account away from `T::owner(program_id)` while
    /// leaving the discriminator + Borsh-compatible payload in place.
    expected_owner: Address,
}

// Forward `Space::INIT_SPACE` from the inner type and add 8 for the
// discriminator. Lets `#[account(init)]` default to the correct size
// for Borsh-backed accounts when `space` is omitted.
impl<T> crate::Space for BorshAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Owner + Discriminator + crate::Space,
{
    const INIT_SPACE: usize = 8 + T::INIT_SPACE;
}

enum BorshBorrow {
    Immutable { _guard: Ref<'static, [u8]> },
    Mutable { guard: RefMut<'static, [u8]> },
    Released,
}

impl<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> BorshAccount<T> {
    /// Returns the account's on-chain address. Inherent method so
    /// `.address()` works uniformly on all wrapper types — `Signer`,
    /// `Account<T>`, `BorshAccount<T>`, `UncheckedAccount`, etc. — without
    /// callers needing to know whether the wrapper derefs to `AccountView`
    /// or to `T`.
    #[inline(always)]
    pub fn address(&self) -> &Address {
        self.view.address()
    }

    /// Release the data borrow guard so the underlying `AccountView` can be
    /// resized or passed to CPIs that call `check_borrow_mut()`. After this,
    /// `exit()` becomes a no-op until `reacquire_borrow_mut()` is called.
    pub fn release_borrow(&mut self) {
        self.borrow = BorshBorrow::Released;
    }

    /// Re-acquire a mutable borrow after a `release_borrow()` + CPI.
    ///
    /// Re-runs the full load-time invariants (owner / size /
    /// discriminator) and re-deserializes `self.data` from the live
    /// buffer, so CPI-induced changes are picked up and a CPI that
    /// reassigned the account or swapped its disc is rejected. Any
    /// in-memory modifications to `self.data` made before
    /// `release_borrow` are dropped — re-apply them after this call
    /// if needed.
    ///
    /// Returns `IllegalOwner` / `AccountDataTooSmall` /
    /// `InvalidAccountData` if the account no longer validates as `T`.
    pub fn reacquire_borrow_mut(&mut self) -> Result<(), ProgramError> {
        // Re-run the load-time invariants. A CPI in the release window
        // could have mutated owner, discriminator, or payload in any
        // combination — without re-checking, we'd accept an account that
        // no longer validates as `T`.
        if !self.view.owned_by(&self.expected_owner) {
            return Err(ProgramError::IllegalOwner);
        }
        let mut view_mut = self.view;
        let data_ref = view_mut.try_borrow_mut()?;
        if data_ref.len() < DISC_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if &data_ref[..DISC_LEN] != T::DISCRIMINATOR {
            return Err(ProgramError::InvalidAccountData);
        }
        self.data = T::deserialize(&mut &data_ref[DISC_LEN..])
            .map_err(|_| ProgramError::InvalidAccountData)?;
        let guard: RefMut<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        self.borrow = BorshBorrow::Mutable { guard };
        Ok(())
    }

    /// Refresh only the borrow guard without touching `self.data`. Used
    /// after a scope-local buffer resize — the `realloc` account
    /// constraint emits this after `realloc_account`, and `exit()`'s
    /// stale-size path calls it too — where the user's in-memory
    /// modifications to `self.data` must be preserved for the subsequent
    /// serialize. Re-deserializing here would read the pre-resize bytes
    /// still on disk, which fails in the shrink case where the stored
    /// Borsh length prefix now exceeds the post-resize buffer.
    ///
    /// For post-CPI use (where CPI may have mutated owner, disc, or
    /// payload), use [`reacquire_borrow_mut`] instead.
    pub fn reacquire_guard_only(&mut self) -> Result<(), ProgramError> {
        let mut view_mut = self.view;
        let data_ref = view_mut.try_borrow_mut()?;
        let guard: RefMut<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        self.borrow = BorshBorrow::Mutable { guard };
        Ok(())
    }

    fn validate_and_load(
        view: AccountView,
        data: &[u8],
        program_id: &Address,
    ) -> Result<T, ProgramError> {
        // Hot path: a single owner check. The "uninitialized placeholder"
        // disambiguation lives in `cold_owner_error` (slab.rs) — see
        // the comment there for why this is safe.
        if !view.owned_by(&T::owner(program_id)) {
            return Err(super::slab::cold_owner_error(&view));
        }
        if data.len() < DISC_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if &data[..DISC_LEN] != T::DISCRIMINATOR {
            return Err(ProgramError::InvalidAccountData);
        }
        T::deserialize(&mut &data[DISC_LEN..]).map_err(|_| ProgramError::InvalidAccountData)
    }
}

impl<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> AnchorAccount
    for BorshAccount<T>
{
    type Data = T;
    const MIN_DATA_LEN: usize = 8;

    fn load(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        let data_ref = view.try_borrow()?;
        let data = Self::validate_and_load(view, &data_ref, program_id)?;
        // SAFETY: AccountView's raw pointer is valid for the entire instruction
        // lifetime (Solana runtime guarantee). We hold the Ref to prevent
        // subsequent mutable borrows on the same account (duplicate detection).
        let guard: Ref<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        Ok(Self {
            view,
            data,
            borrow: BorshBorrow::Immutable { _guard: guard },
            expected_owner: T::owner(program_id),
        })
    }

    /// # Safety
    ///
    /// See [`AnchorAccount::load_mut`] — caller must ensure no other live
    /// `&mut` to the same account data exists.
    unsafe fn load_mut(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        // Guardrail: catches "forgot `#[account(mut)]`" early with a clear
        // error. Under `default-features = false` the Solana runtime still
        // rejects the tx when we try to write, just with a less specific
        // message. Zero CU when compiled out.
        #[cfg(feature = "guardrails")]
        if !view.is_writable() {
            return Err(super::slab::cold_not_writable());
        }
        let mut view_mut = view;
        let data_ref = view_mut.try_borrow_mut()?;
        let data = Self::validate_and_load(view, &data_ref, program_id)?;
        // SAFETY: Same as load(). RefMut provides exclusive access and prevents
        // any other borrow on the same account.
        let guard: RefMut<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        Ok(Self {
            view,
            data,
            borrow: BorshBorrow::Mutable { guard },
            expected_owner: T::owner(program_id),
        })
    }

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

        // Defense-in-depth: write a closed-account sentinel ([u8::MAX; 8])
        // over the discriminator before pinocchio's close() zeros the
        // 48-byte header (lamports + data_len + owner). pinocchio's
        // close does not zero the data region — verified by the
        // `close_zeros_the_48_byte_header` test. If a future caller
        // restores data_len + owner without going through SVM zero-on-
        // allocate, the stale discriminator would otherwise allow a
        // reload with pre-close state. Skipped if the borrow is not
        // mutable (caller programming error, separate concern).
        if let BorshBorrow::Mutable { ref mut guard } = self.borrow {
            if guard.len() >= 8 {
                guard[..8].copy_from_slice(&[u8::MAX; 8]);
            }
        }
        self.borrow = BorshBorrow::Released;

        self_view.close()?;
        Ok(())
    }

    fn exit(&mut self) -> pinocchio::ProgramResult {
        // Skip serialization if account was closed (lamports == 0, reassigned to system program).
        if self.view.lamports() == 0 {
            return Ok(());
        }
        // Belt-and-braces: the derive's `realloc` constraint does
        // release_borrow + reacquire after resize, but if someone resizes
        // through a non-derive path the guard's length would be stale.
        // Detect and fix before serializing.
        let stale = matches!(&self.borrow, BorshBorrow::Mutable { guard } if guard.len() != self.view.data_len());
        if stale {
            self.release_borrow();
            // Use reacquire_guard_only here — we do NOT want to re-read
            // self.data from the (potentially just-resized) buffer.
            // The user's in-memory modifications to self.data are what
            // we're about to serialize.
            self.reacquire_guard_only()?;
        }
        if let BorshBorrow::Mutable { ref mut guard } = self.borrow {
            self.data
                .serialize(&mut &mut guard[DISC_LEN..])
                .map_err(|_| ProgramError::InvalidAccountData)?;
        }
        Ok(())
    }
}

impl<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> Deref for BorshAccount<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.data
    }
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

impl<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> AsRef<AccountView>
    for BorshAccount<T>
{
    fn as_ref(&self) -> &AccountView {
        &self.view
    }
}

/// Forward `Discriminator` from a `BorshAccount<T>` to its inner type. Lets
/// the `#[account(zeroed)]` derive codegen look up the disc via the field type
/// directly (`<BorshAccount<Counter> as Discriminator>::DISCRIMINATOR`).
impl<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> Discriminator
    for BorshAccount<T>
{
    const DISCRIMINATOR: &'static [u8] = T::DISCRIMINATOR;
}

#[cfg(feature = "idl-build")]
impl<T> crate::IdlAccountType for BorshAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Owner + Discriminator + crate::IdlAccountType,
{
    const __IDL_TYPE: Option<&'static str> = T::__IDL_TYPE;
    fn __register_idl_deps(types: &mut ::alloc::vec::Vec<&'static str>) {
        T::__register_idl_deps(types);
    }
}

/// Init for `BorshAccount<T>`: creates the account, writes the discriminator,
/// then borsh-deserializes `T` from the zero-filled tail. Types whose borsh
/// schema rejects all-zero encoding cannot be `init`-ed this way.
impl<T> AccountInitialize for BorshAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Owner + Discriminator,
{
    type Params<'a> = ();

    #[inline(always)]
    fn create_and_initialize<'a>(
        payer: &AccountView,
        account: &AccountView,
        space: usize,
        program_id: &Address,
        _params: &(),
        signer_seeds: Option<&[&[u8]]>,
    ) -> Result<Self, ProgramError> {
        let disc: &[u8; 8] = T::DISCRIMINATOR
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?;
        match signer_seeds {
            Some(seeds) => crate::create_account_signed(payer, account, space, program_id, seeds)?,
            None => crate::create_account(payer, account, space, program_id)?,
        }
        let mut view_mut = *account;
        let data_ref = view_mut.try_borrow_mut()?;
        let mut guard: RefMut<'static, [u8]> = unsafe { core::mem::transmute(data_ref) };
        match guard.first_chunk_mut::<DISC_LEN>() {
            Some(dst) => *dst = *disc,
            None => return Err(ProgramError::AccountDataTooSmall),
        }
        let data = T::deserialize(&mut &guard[DISC_LEN..])
            .map_err(|_| ProgramError::InvalidAccountData)?;
        Ok(Self {
            view: *account,
            data,
            borrow: BorshBorrow::Mutable { guard },
            expected_owner: T::owner(program_id),
        })
    }
}
