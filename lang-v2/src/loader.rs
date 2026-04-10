use {
    pinocchio::{
        account::AccountView,
        address::Address,
    },
    solana_program_error::ProgramError,
    crate::{
        AnchorAccount, ErrorCode,
        accounts::AccountInitialize,
        cpi::find_program_address,
    },
};

/// Sequential account loader for `#[derive(Accounts)]`.
///
/// Wraps the accounts slice and an advancing index so that the macro only
/// needs to emit one `loader.next*()` call per field.
pub struct AccountLoader<'a> {
    program_id: &'a Address,
    accounts: &'a [AccountView],
    idx: usize,
}

impl<'a> AccountLoader<'a> {
    pub fn new(program_id: &'a Address, accounts: &'a [AccountView]) -> Self {
        Self { program_id, accounts, idx: 0 }
    }

    pub fn consumed(&self) -> usize { self.idx }

    fn peek(&mut self) -> Result<AccountView, ProgramError> {
        let view = *self.accounts.get(self.idx)
            .ok_or(ProgramError::from(ErrorCode::AccountNotEnoughKeys))?;
        self.idx += 1;
        Ok(view)
    }

    // -- Load --

    pub fn next<T: AnchorAccount>(&mut self) -> Result<T, ProgramError> {
        T::load(self.peek()?, self.program_id)
    }

    pub fn next_mut<T: AnchorAccount>(&mut self) -> Result<T, ProgramError> {
        T::load_mut(self.peek()?, self.program_id)
    }

    // -- Init (uses AccountInitialize) --

    /// Create + init via `AccountInitialize`. No PDA.
    pub fn next_init<T: AccountInitialize>(
        &mut self,
        payer: &AccountView,
        space: usize,
        params: &T::Params<'_>,
    ) -> Result<AccountView, ProgramError> {
        let view = self.peek()?;
        T::create_and_initialize(payer, &view, space, self.program_id, params, None)?;
        Ok(view)
    }

    /// Create + init PDA via `AccountInitialize`. Returns `(view, bump)`.
    pub fn next_init_pda<T: AccountInitialize>(
        &mut self,
        payer: &AccountView,
        space: usize,
        seeds: &[&[u8]],
        params: &T::Params<'_>,
    ) -> Result<(AccountView, u8), ProgramError> {
        let view = self.peek()?;
        let (pda, bump) = find_program_address(seeds, self.program_id);
        if *view.address() != pda {
            return Err(ErrorCode::ConstraintSeeds.into());
        }
        // Build signer seeds with bump appended
        assert!(seeds.len() <= 16, "PDA seeds exceed maximum of 16");
        let bump_byte: [u8; 1] = [bump];
        let mut full_seeds: [&[u8]; 17] = [&[]; 17];
        for (i, s) in seeds.iter().enumerate() { full_seeds[i] = *s; }
        full_seeds[seeds.len()] = &bump_byte;
        let signer_seeds = &full_seeds[..seeds.len() + 1];

        T::create_and_initialize(payer, &view, space, self.program_id, params, Some(signer_seeds))?;
        Ok((view, bump))
    }

    // -- Init-if-needed --

    /// Init if not already initialized, otherwise return the view for load_mut.
    /// Returns `(view, already_initialized)`.
    pub fn next_init_if_needed<T: AccountInitialize>(
        &mut self,
        payer: &AccountView,
        space: usize,
        params: &T::Params<'_>,
    ) -> Result<(AccountView, bool), ProgramError> {
        let view = self.peek()?;
        let already_init = view.owned_by(self.program_id) && view.data_len() > 0;
        if !already_init {
            T::create_and_initialize(payer, &view, space, self.program_id, params, None)?;
        }
        Ok((view, already_init))
    }

    /// Init-if-needed PDA. Returns `(view, bump, already_initialized)`.
    pub fn next_init_if_needed_pda<T: AccountInitialize>(
        &mut self,
        payer: &AccountView,
        space: usize,
        seeds: &[&[u8]],
        params: &T::Params<'_>,
    ) -> Result<(AccountView, u8, bool), ProgramError> {
        let view = self.peek()?;
        let (pda, bump) = find_program_address(seeds, self.program_id);
        if *view.address() != pda {
            return Err(ErrorCode::ConstraintSeeds.into());
        }
        let already_init = view.owned_by(self.program_id) && view.data_len() > 0;
        if !already_init {
            assert!(seeds.len() <= 16, "PDA seeds exceed maximum of 16");
            let bump_byte: [u8; 1] = [bump];
            let mut full_seeds: [&[u8]; 17] = [&[]; 17];
            for (i, s) in seeds.iter().enumerate() { full_seeds[i] = *s; }
            full_seeds[seeds.len()] = &bump_byte;
            let signer_seeds = &full_seeds[..seeds.len() + 1];

            T::create_and_initialize(payer, &view, space, self.program_id, params, Some(signer_seeds))?;
        }
        Ok((view, bump, already_init))
    }
}
