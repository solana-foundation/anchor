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
/// Validates owner, checks discriminator, deserializes via borsh.
/// Holds a pinocchio borrow guard to prevent duplicate mutable accounts:
/// - `load()` takes an immutable borrow (blocks subsequent `load_mut` on same account)
/// - `load_mut()` takes a mutable borrow (blocks any other borrow on same account)
/// - `exit()` serializes through the held `RefMut`
///
/// ## `#[account(owner = X @ MyErr)]` does NOT surface `MyErr`
///
/// `BorshAccount<T>` validates the on-chain owner against `T::owner(program_id)`
/// as part of `load`/`load_mut` — that check runs *before* any derive-level
/// constraint hook. A mismatch surfaces as `ProgramError::IllegalOwner`, never
/// as the user's `@ MyErr` code. The same is true for the discriminator check.
///
/// If you need a custom error code on owner mismatch, switch the field to
/// `UncheckedAccount` (which has no built-in owner validation) and the
/// derive-level `owner = X @ MyErr` becomes the authoritative check. Note
/// that you also lose the discriminator and borsh deserialization the wrapper
/// would have done — make those checks explicit in the handler if you need
/// them.
pub struct BorshAccount<T: BorshDeserialize + BorshSerialize + Owner + Discriminator> {
    view: AccountView,
    data: T,
    borrow: BorshBorrow,
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
    /// Release the data borrow guard so the underlying `AccountView` can be
    /// resized or passed to CPIs that call `check_borrow_mut()`. After this,
    /// `exit()` becomes a no-op until `reacquire_borrow_mut()` is called.
    pub fn release_borrow(&mut self) {
        self.borrow = BorshBorrow::Released;
    }

    /// Re-acquire a mutable borrow after a `release_borrow()` + resize/CPI.
    /// The underlying buffer may have changed size — any subsequent exit()
    /// will serialize through the fresh RefMut.
    pub fn reacquire_borrow_mut(&mut self) -> Result<(), ProgramError> {
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
        // disambiguation lives in `cold_owner_error` (account.rs) — see
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
        })
    }

    fn account(&self) -> &AccountView {
        &self.view
    }

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
/// the `#[account(zero)]` derive codegen look up the disc via the field type
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

/// Wrapper-level init for `BorshAccount<T>`: creates the underlying account
/// (CPI to system program), writes the 8-byte discriminator, then borsh-
/// deserializes `T` from the zero-filled tail so the in-memory state matches
/// exactly what a subsequent `load()` would observe. Any deserialization
/// failure propagates — types whose borsh schema rejects all-zero encoding
/// cannot be `init`-ed this way and must be constructed through a custom
/// handler.
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
        })
    }
}
