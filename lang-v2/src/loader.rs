use {
    crate::{cursor::AccountCursor, AnchorAccount},
    pinocchio::{account::AccountView, address::Address},
    solana_program_error::ProgramError,
};

/// Sequential account loader for `#[derive(Accounts)]`.
///
/// Thin wrapper around an [`AccountCursor`] and a borrowed `program_id`.
/// The macro emits one `loader.next*()` call per declared field and the
/// loader forwards to the underlying cursor, which walks the serialized
/// input buffer on demand.
///
/// Bounds checking is not re-done per field: the dispatcher has already
/// verified `num_accounts >= T::HEADER_SIZE` before constructing the
/// cursor, so each `next*()` call is safe to unwrap.
pub struct AccountLoader<'a> {
    program_id: &'a Address,
    cursor: &'a mut AccountCursor,
}

impl<'a> AccountLoader<'a> {
    #[inline(always)]
    pub fn new(program_id: &'a Address, cursor: &'a mut AccountCursor) -> Self {
        Self { program_id, cursor }
    }

    #[inline(always)]
    pub fn consumed(&self) -> u8 {
        self.cursor.consumed()
    }

    /// Walk one account from the cursor and return the raw `AccountView`.
    ///
    /// Used by the init / init_if_needed / zeroed codegen paths where
    /// the derive macro wants to construct + validate the account itself.
    #[inline(always)]
    pub fn next_view(&mut self) -> Result<AccountView, ProgramError> {
        // SAFETY: dispatcher has bounds-checked num_accounts >= HEADER_SIZE.
        Ok(unsafe { self.cursor.next() })
    }

    /// Walk + `T::load()` the next account.
    #[inline(always)]
    pub fn next<T: AnchorAccount>(&mut self) -> Result<T, ProgramError> {
        let view = unsafe { self.cursor.next() };
        T::load(view, self.program_id)
    }

    /// Walk + `T::load_mut()` the next account.
    #[inline(always)]
    pub fn next_mut<T: AnchorAccount>(&mut self) -> Result<T, ProgramError> {
        let view = unsafe { self.cursor.next() };
        T::load_mut(view, self.program_id)
    }
}
