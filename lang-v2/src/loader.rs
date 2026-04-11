use {
    pinocchio::{
        account::AccountView,
        address::Address,
    },
    solana_program_error::ProgramError,
    crate::{AnchorAccount, ErrorCode},
};

/// Sequential account loader for `#[derive(Accounts)]`.
///
/// Wraps the accounts slice and an advancing index so that the macro only
/// needs to emit one `loader.next*()` call per field.
///
/// TODO: reassess whether this indirection still pays for itself. The macro
/// could generate `<#field_ty>::load(accounts[N], program_id)?` directly with
/// compile-time indices and a single bounds check per struct. The loader was
/// useful when there were more variants (next_init_pda, init_if_needed, etc.)
/// but after removing those helpers it's a thin shim around slice indexing.
pub struct AccountLoader<'a> {
    program_id: &'a Address,
    accounts: &'a [AccountView],
    idx: usize,
}

impl<'a> AccountLoader<'a> {
    #[inline(always)]
    pub fn new(program_id: &'a Address, accounts: &'a [AccountView]) -> Self {
        Self { program_id, accounts, idx: 0 }
    }

    #[inline(always)]
    pub fn consumed(&self) -> usize { self.idx }

    #[inline(always)]
    fn peek(&mut self) -> Result<AccountView, ProgramError> {
        let view = *self.accounts.get(self.idx)
            .ok_or(ProgramError::from(ErrorCode::AccountNotEnoughKeys))?;
        self.idx += 1;
        Ok(view)
    }

    /// Consume the next account and return its raw view.
    /// Used by init codegen which handles creation + loading itself.
    #[inline(always)]
    pub fn next_view(&mut self) -> Result<AccountView, ProgramError> {
        self.peek()
    }

    #[inline(always)]
    pub fn next<T: AnchorAccount>(&mut self) -> Result<T, ProgramError> {
        T::load(self.peek()?, self.program_id)
    }

    #[inline(always)]
    pub fn next_mut<T: AnchorAccount>(&mut self) -> Result<T, ProgramError> {
        T::load_mut(self.peek()?, self.program_id)
    }
}
