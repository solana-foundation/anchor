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

    fn load(view: AccountView, program_id: &Address) -> core::result::Result<Self, ProgramError>;

    /// Default impl: validates `is_writable`, then delegates to `load()`.
    ///
    /// Zero-data view wrappers (`UncheckedAccount`, `SystemAccount`,
    /// `Program<T>`, `Sysvar<T>`) inherit this default, so every
    /// `#[account(mut)] pub x: X` gets a writable check without the
    /// derive having to emit a separate constraint block. This moves
    /// the check from the derive's constraints list into the trait
    /// itself — single place, single rule.
    ///
    /// Data-carrying wrappers (`Account<T>`, `BorshAccount<T>`,
    /// `Slab<H, T>`) override this to acquire a mutable borrow via
    /// pinocchio's borrow tracking AND perform the writable check
    /// themselves (currently gated behind the `guardrails` feature
    /// for efficiency).
    ///
    /// `Signer` overrides this with a fused 2-byte `is_signer` +
    /// `is_writable` check — see `accounts/signer.rs`.
    #[inline(always)]
    fn load_mut(view: AccountView, program_id: &Address) -> core::result::Result<Self, ProgramError> {
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
    /// wrappers override this to skip the redundant validation and save
    /// the redundant validation.
    ///
    /// [`load_mut`]: Self::load_mut
    #[inline(always)]
    fn load_mut_after_init(
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

/// A constraint check on an account. Each account type opts in to specific
/// constraint keys by implementing this trait for the corresponding marker.
///
/// `V` is the expected value type — defaults to `Address` for address comparisons.
/// Use a different `V` for non-address checks (e.g. `mint::DecimalsConstraint` uses `u8`).
///
/// Unknown keys → compile error ("Constrain<X> is not implemented for Y").
pub trait Constrain<C, V = Address> {
    fn constrain(&self, expected: &V) -> core::result::Result<(), ProgramError>;
}

pub struct Nested<T>(pub T);

impl<T> Deref for Nested<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.0 }
}

impl<T> core::ops::DerefMut for Nested<T> {
    fn deref_mut(&mut self) -> &mut T { &mut self.0 }
}
