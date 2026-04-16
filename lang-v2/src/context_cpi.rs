extern crate alloc;

use {
    crate::{CpiHandle, ToCpiAccounts},
    alloc::vec::Vec,
    pinocchio::{
        address::Address,
        instruction::{InstructionAccount, InstructionView},
    },
    solana_program_error::ProgramError,
};

/// Context for cross-program invocations.
///
/// Bundles a typed CPI accounts struct with the target program, optional
/// PDA signer seeds, and optional remaining accounts.
///
/// # Example
///
/// ```ignore
/// let cpi_accounts = callee::cpi::accounts::SetData {
///     data_acc: ctx.accounts.data_acc.cpi_handle_mut(),
///     authority: ctx.accounts.authority.cpi_handle(),
/// };
/// let cpi_ctx = CpiContext::new(ctx.accounts.callee.address(), cpi_accounts);
/// callee::cpi::set_data(cpi_ctx, data)?;
/// ```
pub struct CpiContext<'a, T: ToCpiAccounts<'a>> {
    pub accounts: T,
    pub remaining_accounts: Vec<CpiHandle<'a>>,
    pub program: &'a Address,
    pub signer_seeds: &'a [&'a [&'a [u8]]],
}

impl<'a, T: ToCpiAccounts<'a>> CpiContext<'a, T> {
    #[must_use]
    pub fn new(program: &'a Address, accounts: T) -> Self {
        Self {
            accounts,
            program,
            remaining_accounts: Vec::new(),
            signer_seeds: &[],
        }
    }

    #[must_use]
    pub fn new_with_signer(
        program: &'a Address,
        accounts: T,
        signer_seeds: &'a [&'a [&'a [u8]]],
    ) -> Self {
        Self {
            accounts,
            program,
            remaining_accounts: Vec::new(),
            signer_seeds,
        }
    }

    #[must_use]
    pub fn with_signer(mut self, signer_seeds: &'a [&'a [&'a [u8]]]) -> Self {
        self.signer_seeds = signer_seeds;
        self
    }

    #[must_use]
    pub fn with_remaining_accounts(mut self, ra: Vec<CpiHandle<'a>>) -> Self {
        self.remaining_accounts = ra;
        self
    }

    /// Invoke the CPI with the given instruction data.
    ///
    /// Collects instruction accounts and CPI handles from the typed
    /// accounts struct (via [`ToCpiAccounts`]), appends remaining
    /// accounts, converts signer seeds to pinocchio's format, and
    /// calls `invoke_signed_unchecked`.
    pub fn invoke(&self, data: &[u8]) -> Result<(), ProgramError> {
        let mut instruction_accounts = self.accounts.to_instruction_accounts();
        let mut handles = self.accounts.to_cpi_handles();

        // Append remaining accounts.
        for handle in &self.remaining_accounts {
            let view = handle.account_view();
            instruction_accounts.push(InstructionAccount::new(
                view.address(),
                view.is_writable(),
                view.is_signer(),
            ));
            handles.push(*handle);
        }

        let instruction = InstructionView {
            program_id: self.program,
            data,
            accounts: &instruction_accounts,
        };

        // Convert signer seeds to pinocchio Signers.
        // SAFETY: pinocchio::cpi::Seed is repr(C) { *const u8, u64, PhantomData }
        // which has the same layout as &[u8] on SBF. This is verified by the
        // static assertion in cpi.rs.
        let signers: Vec<pinocchio::cpi::Signer> = self
            .signer_seeds
            .iter()
            .map(|seeds| {
                let cpi_seeds: &[pinocchio::cpi::Seed] = unsafe {
                    core::slice::from_raw_parts(
                        seeds.as_ptr() as *const pinocchio::cpi::Seed,
                        seeds.len(),
                    )
                };
                pinocchio::cpi::Signer::from(cpi_seeds)
            })
            .collect();

        // Build CpiAccounts and invoke.
        let n = handles.len();
        let mut cpi_accounts: Vec<core::mem::MaybeUninit<pinocchio::cpi::CpiAccount>> =
            Vec::with_capacity(n);
        // SAFETY: MaybeUninit does not require initialization.
        unsafe { cpi_accounts.set_len(n) };

        for (handle, slot) in handles.iter().zip(cpi_accounts.iter_mut()) {
            pinocchio::cpi::CpiAccount::init_from_account_view(handle.account_view(), slot);
        }

        // SAFETY:
        // - All CpiAccounts initialized by init_from_account_view above.
        // - CpiHandles hold Rust borrows preventing typed data access.
        // - Pinocchio borrow_state remains set, blocking raw borrows on
        //   stale AccountView copies.
        unsafe {
            pinocchio::cpi::invoke_signed_unchecked(
                &instruction,
                core::slice::from_raw_parts(
                    cpi_accounts.as_ptr() as *const pinocchio::cpi::CpiAccount,
                    n,
                ),
                &signers,
            );
        }

        Ok(())
    }
}
