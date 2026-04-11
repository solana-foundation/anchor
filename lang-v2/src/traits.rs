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
    fn load_mut(view: AccountView, program_id: &Address) -> core::result::Result<Self, ProgramError>;
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
