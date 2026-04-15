use {
    core::ops::Deref,
    pinocchio::{
        account::AccountView,
        address::Address,
    },
    solana_program_error::{ProgramError, ProgramResult},
};

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

    fn exit(&mut self) -> ProgramResult { Ok(()) }

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
    fn deref(&self) -> &T { &self.0 }
}

impl<T> core::ops::DerefMut for Nested<T> {
    fn deref_mut(&mut self) -> &mut T { &mut self.0 }
}
