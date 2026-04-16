extern crate alloc;

use {
    crate::{AccountInitialize, AnchorAccount, Constrain, Discriminator, Space},
    alloc::boxed::Box,
    pinocchio::{account::AccountView, address::Address},
    solana_program_error::ProgramError,
};

impl<T: AnchorAccount> AnchorAccount for Box<T> {
    type Data = T;
    const MIN_DATA_LEN: usize = T::MIN_DATA_LEN;

    fn load(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        T::load(view, program_id).map(Box::new)
    }

    /// # Safety
    ///
    /// See [`AnchorAccount::load_mut`] — caller must ensure no other live
    /// `&mut` to the same account data exists.
    unsafe fn load_mut(view: AccountView, program_id: &Address) -> Result<Self, ProgramError> {
        T::load_mut(view, program_id).map(Box::new)
    }

    /// # Safety
    ///
    /// See [`AnchorAccount::load_mut_after_init`] — caller must ensure no
    /// other live `&mut` to the same account data exists.
    unsafe fn load_mut_after_init(
        view: AccountView,
        program_id: &Address,
    ) -> Result<Self, ProgramError> {
        T::load_mut_after_init(view, program_id).map(Box::new)
    }

    fn account(&self) -> &AccountView {
        (**self).account()
    }

    fn exit(&mut self) -> pinocchio::ProgramResult {
        (**self).exit()
    }

    fn close(&mut self, destination: AccountView) -> pinocchio::ProgramResult {
        (**self).close(destination)
    }
}

#[cfg(feature = "idl-build")]
impl<T: crate::IdlAccountType> crate::IdlAccountType for Box<T> {
    const __IDL_TYPE: Option<&'static str> = T::__IDL_TYPE;
    fn __register_idl_deps(types: &mut ::alloc::vec::Vec<&'static str>) {
        T::__register_idl_deps(types);
    }
}

// ---------------------------------------------------------------------------
// Forward the init-time trait surface so `Box<Account<T>>` and
// `Box<BorshAccount<T>>` work with `#[account(init, …)]`, `#[account(zeroed)]`,
// `space = …` omitted, and namespaced constraints (`token::mint = …`, etc.).
//
// The derive reaches for these traits via UFCS on the field type — e.g.
// `<Box<Account<T>> as AccountInitialize>::create_and_initialize(…)` — so
// auto-deref on the receiver isn't sufficient; explicit forwards are required.
// ---------------------------------------------------------------------------

impl<T: AccountInitialize> AccountInitialize for Box<T> {
    type Params<'a> = T::Params<'a>;

    fn create_and_initialize<'a>(
        payer: &AccountView,
        account: &AccountView,
        space: usize,
        program_id: &Address,
        params: &Self::Params<'a>,
        signer_seeds: Option<&[&[u8]]>,
    ) -> Result<Self, ProgramError> {
        T::create_and_initialize(payer, account, space, program_id, params, signer_seeds)
            .map(Box::new)
    }
}

impl<T: Space> Space for Box<T> {
    const INIT_SPACE: usize = T::INIT_SPACE;
}

impl<T: Discriminator> Discriminator for Box<T> {
    const DISCRIMINATOR: &'static [u8] = T::DISCRIMINATOR;
}

impl<T, C, V> Constrain<C, V> for Box<T>
where
    T: Constrain<C, V>,
{
    fn constrain(&mut self, expected: &V) -> Result<(), ProgramError> {
        (**self).constrain(expected)
    }
}
