use {
    core::ops::Deref,
    pinocchio::{
        account::AccountView,
        address::Address,
    },
    solana_program_error::{ProgramError, ProgramResult},
};

/// Discriminator length in bytes.
pub const DISC_LEN: usize = 8;

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

pub trait Owner {
    fn owner() -> Address;
}

pub trait Id {
    fn id() -> Address;
}

pub trait Discriminator {
    const DISCRIMINATOR: &'static [u8];
}

pub struct Nested<T>(pub T);

impl<T> Deref for Nested<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.0 }
}

impl<T> core::ops::DerefMut for Nested<T> {
    fn deref_mut(&mut self) -> &mut T { &mut self.0 }
}
