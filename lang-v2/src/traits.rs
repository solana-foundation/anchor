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
    writable: bool,
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

    /// Whether this handle was obtained via `cpi_handle_mut`.
    #[inline(always)]
    pub fn is_writable(&self) -> bool {
        self.writable
    }

    /// Whether the underlying account is a signer on the transaction.
    #[inline(always)]
    pub fn is_signer(&self) -> bool {
        self.view.is_signer()
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

    /// Minimum account data length for this type. When > 0, PDA
    /// verification can skip `sol_curve_validate_point`: a non-empty
    /// account was created via CreateAccount/Allocate (which requires
    /// signing), and `invoke_signed` already includes the curve check.
    ///
    /// Slab-backed types: `8`. UncheckedAccount / zero-data wrappers: `0`
    /// (forces the curve check).
    const MIN_DATA_LEN: usize = 0;

    fn load(view: AccountView, program_id: &Address) -> core::result::Result<Self, ProgramError>;

    /// Load an account for mutable access.
    ///
    /// # Safety
    ///
    /// No other live `&mut` to the same account data may exist while the
    /// returned value is alive. In derive-generated code the bitvec
    /// duplicate-account check enforces this; direct callers must uphold
    /// it themselves.
    ///
    /// Default impl validates `is_writable` and delegates to `load()`.
    /// Data-carrying wrappers (`Account<T>`, `BorshAccount<T>`, `Slab<H, T>`)
    /// override to use `borrow_unchecked_mut` for write provenance.
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

    /// Like [`load_mut`], but called right after
    /// `AccountInitialize::create_and_initialize`. Owner, discriminator,
    /// and min-length checks are tautologies on this path, so data-carrying
    /// wrappers override to skip them. Default forwards to [`load_mut`].
    ///
    /// # Safety
    ///
    /// Same as [`load_mut`]: no other live `&mut` to the same account data.
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

    /// Obtain a read-only CPI handle for this account.
    ///
    /// The handle borrows `self`, preventing mutable typed access while
    /// it is alive. The handle's `is_writable` flag is `false`.
    #[inline(always)]
    fn cpi_handle(&self) -> CpiHandle<'_> {
        CpiHandle {
            view: self.account(),
            writable: false,
        }
    }

    /// Obtain a writable CPI handle for this account.
    ///
    /// The handle borrows `self` mutably, preventing any typed access
    /// while it is alive. The handle's `is_writable` flag is `true`.
    ///
    /// # Panics
    ///
    /// Panics if the underlying account is not marked writable in
    /// the transaction.
    #[inline(always)]
    fn cpi_handle_mut(&mut self) -> CpiHandle<'_> {
        // Unconditional (not guardrails-gated): passing a read-only account
        // to a CPI that writes is a program bug, not a "nice to have" check.
        assert!(
            self.account().is_writable(),
            "cpi_handle_mut called on a read-only account"
        );
        CpiHandle {
            view: self.account(),
            writable: true,
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
    /// Well-known base58 program address for IDL emission. Empty string
    /// signals "no address to advertise in the IDL" — consumed by
    /// `IdlAccountType::__IDL_ADDRESS` on `Program<T>` and converted to
    /// `None` there.
    #[cfg(feature = "idl-build")]
    const IDL_ADDRESS: &'static str = "";
}

pub trait Discriminator {
    const DISCRIMINATOR: &'static [u8];
}

/// Wrapper-level init: creates the on-chain account and returns a loaded
/// `Self`. `Slab<H, T>` and `BorshAccount<T>` get this automatically;
/// custom wrappers implement it directly.
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

/// Constraint check on an account. `V` defaults to `Address`; override
/// for non-address checks (e.g. `mint::DecimalsConstraint` uses `u8`).
/// Unknown keys produce a compile error.
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

#[cfg(feature = "idl-build")]
impl<T: crate::IdlAccountType> crate::IdlAccountType for Nested<T> {
    const __IDL_TYPE: Option<&'static str> = T::__IDL_TYPE;
    fn __register_idl_deps(types: &mut ::alloc::vec::Vec<&'static str>) {
        T::__register_idl_deps(types);
    }
}
