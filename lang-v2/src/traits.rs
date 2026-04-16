use {
    core::ops::Deref,
    pinocchio::{account::AccountView, address::Address, instruction::InstructionAccount},
    solana_program_error::{ProgramError, ProgramResult},
};

/// Zero-cost CPI handle that borrows an anchor account at the Rust level.
///
/// Obtained via [`AnchorAccount::cpi_handle`] (shared borrow) or
/// [`AnchorAccount::cpi_handle_mut`] (exclusive borrow). Pinocchio's
/// `borrow_state` is never modified — CPI is routed through
/// `invoke_signed_unchecked` by [`CpiContext::invoke`].
///
/// Deliberately does NOT implement `Deref<Target = AccountView>` to
/// prevent accidental use with pinocchio's checked invoke builders.
#[derive(Clone, Copy)]
pub struct CpiHandle<'a> {
    view: &'a AccountView,
}

impl<'a> CpiHandle<'a> {
    /// The account's on-chain address.
    ///
    /// Returns a reference with the inner `'a` lifetime so callers can
    /// build `InstructionAccount<'a>` values without tying the result to
    /// the borrow of `&self`.
    #[inline(always)]
    pub fn address(&self) -> &'a Address {
        self.view.address()
    }

    /// Access the underlying `AccountView` for CPI account construction.
    ///
    /// Restricted to the crate so external code cannot extract the view
    /// and pass it to pinocchio's checked invoke.
    #[inline(always)]
    pub(crate) fn account_view(&self) -> &'a AccountView {
        self.view
    }
}

/// Converts a CPI accounts struct into instruction metadata and handles.
///
/// Implemented by generated CPI accounts structs. Each field maps to an
/// [`InstructionAccount`] (address + writable/signer flags) and a
/// [`CpiHandle`] for the actual invocation.
pub trait ToCpiAccounts<'a> {
    /// Produce instruction account metadata for the CPI instruction.
    fn to_instruction_accounts(&self) -> alloc::vec::Vec<InstructionAccount<'a>>;

    /// Collect all CPI handles for the invocation.
    fn to_cpi_handles(&self) -> alloc::vec::Vec<CpiHandle<'a>>;
}

pub trait AnchorAccount: Deref<Target = Self::Data> + Sized {
    type Data;

    /// Minimum account data length for this type. Used at compile time to
    /// decide whether PDA verification can skip the `sol_curve_validate_point`
    /// syscall: any account with `data_len > 0` must have been signed for
    /// (via CreateAccount/Allocate), and signing proves PDA validity because
    /// `invoke_signed` → `create_program_address` includes the runtime's own
    /// curve check.
    ///
    /// Slab-backed types override this to `8` (discriminator length).
    /// UncheckedAccount and other zero-data wrappers keep the default `0`,
    /// which forces the curve check on PDA verification.
    const MIN_DATA_LEN: usize = 0;

    fn load(view: AccountView, program_id: &Address) -> core::result::Result<Self, ProgramError>;

    /// Load an account for mutable access.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that no other live `&mut` reference to the
    /// same account's data exists for the duration that the returned value
    /// (or any `&mut` derived from it) is alive. Violating this creates two
    /// `&mut` references aliasing the same memory, which is immediate
    /// undefined behaviour under Rust's aliasing rules.
    ///
    /// In generated `#[derive(Accounts)]` code this invariant is upheld by
    /// the bitvec duplicate-account check that fires before any handler code
    /// runs. Direct callers are responsible for the same guarantee.
    ///
    /// ## Implementing types
    ///
    /// Zero-data view wrappers (`UncheckedAccount`, `SystemAccount`,
    /// `Program<T>`, `Sysvar<T>`) inherit this default, which validates
    /// `is_writable` and delegates to `load()`. Data-carrying wrappers
    /// (`Account<T>`, `BorshAccount<T>`, `Slab<H, T>`) override this to
    /// use `borrow_unchecked_mut` for zero-cost access with write provenance.
    /// `Signer` overrides with a fused `is_signer` + `is_writable` check.
    #[inline(always)]
    unsafe fn load_mut(
        view: AccountView,
        program_id: &Address,
    ) -> core::result::Result<Self, ProgramError> {
        if !view.is_writable() {
            return Err(crate::ErrorCode::ConstraintMut.into());
        }
        Self::load(view, program_id)
    }

    /// Like [`load_mut`], but called immediately after
    /// `AccountInitialize::create_and_initialize`. Owner / discriminator /
    /// minimum-length checks are tautologies on this path:
    /// - the system program set the owner to our program in the CPI
    /// - we just wrote the 8-byte discriminator ourselves
    /// - `create_account` allocated exactly `space = disc + size_of::<T>()`
    ///
    /// Default forwards to [`load_mut`] so every type that doesn't care
    /// (Sysvar, Signer, Program, …) keeps the same behavior. Data-carrying
    /// wrappers override this to skip the redundant validation.
    ///
    /// # Safety
    ///
    /// Same precondition as [`load_mut`]: no other live `&mut` to the same
    /// account data may exist while the returned value is alive.
    ///
    /// [`load_mut`]: Self::load_mut
    #[inline(always)]
    unsafe fn load_mut_after_init(
        view: AccountView,
        program_id: &Address,
    ) -> core::result::Result<Self, ProgramError> {
        Self::load_mut(view, program_id)
    }

    fn account(&self) -> &AccountView;

    fn exit(&mut self) -> ProgramResult {
        Ok(())
    }

    fn close(&mut self, mut destination: AccountView) -> ProgramResult {
        let mut self_view = *self.account();
        let dest_lamports = destination
            .lamports()
            .checked_add(self_view.lamports())
            .ok_or(ProgramError::ArithmeticOverflow)?;
        destination.set_lamports(dest_lamports);
        self_view.set_lamports(0);
        self_view.close()?;
        Ok(())
    }

    /// Obtain a shared CPI handle for this account.
    ///
    /// The handle borrows `self`, preventing mutable typed access while
    /// it is alive. Use for accounts that are read-only in the CPI.
    #[inline(always)]
    fn cpi_handle(&self) -> CpiHandle<'_> {
        CpiHandle {
            view: self.account(),
        }
    }

    /// Obtain an exclusive CPI handle for this account.
    ///
    /// The handle borrows `self` mutably, preventing any typed access
    /// while it is alive. Use for accounts that are writable in the CPI.
    #[inline(always)]
    fn cpi_handle_mut(&mut self) -> CpiHandle<'_> {
        CpiHandle {
            view: self.account(),
        }
    }
}

/// Declares which program owns accounts of this data type.
///
/// For your own program's types, `#[account]` generates this automatically
/// returning `*program_id` (no `declare_id!` needed).
///
/// External crates implement this with their program's address:
/// ```ignore
/// impl Owner for TokenAccountData {
///     fn owner(_program_id: &Address) -> Address { Token::id() }
/// }
/// ```
pub trait Owner {
    fn owner(program_id: &Address) -> Address;
}

pub trait Id {
    fn id() -> Address;
}

pub trait Discriminator {
    const DISCRIMINATOR: &'static [u8];
}

/// Wrapper-level init: creates the on-chain account *and* returns a loaded
/// `Self`. Implementers can capture init-time context (e.g. cache a payer
/// for later tail mutations) between the byte-write and the load.
///
/// `Slab<H, T>` (= `Account<H>` / `BorshAccount<H>`) gets this automatically
/// for any `H: SlabInit` via a forward impl in `accounts/slab.rs`.
/// Self-contained wrappers (custom `AnchorAccount` types that aren't
/// `Slab`-backed) implement it directly.
pub trait AccountInitialize: Sized {
    type Params<'a>: Default;

    fn create_and_initialize<'a>(
        payer: &AccountView,
        account: &AccountView,
        space: usize,
        program_id: &Address,
        params: &Self::Params<'a>,
        signer_seeds: Option<&[&[u8]]>,
    ) -> Result<Self, ProgramError>;
}

/// A constraint check on an account. Each account type opts in to specific
/// constraint keys by implementing this trait for the corresponding marker.
///
/// `V` is the expected value type — defaults to `Address` for address comparisons.
/// Use a different `V` for non-address checks (e.g. `mint::DecimalsConstraint` uses `u8`).
///
/// Unknown keys → compile error ("Constrain<X> is not implemented for Y").
pub trait Constrain<C, V = Address> {
    fn constrain(&mut self, expected: &V) -> core::result::Result<(), ProgramError>;
}

pub struct Nested<T>(pub T);

impl<T> Deref for Nested<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> core::ops::DerefMut for Nested<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}
