//! Common helpers for the derive-macro test-suite.

use anchor_lang::prelude::*;
use anchor_lang::Bumps;

// -----------------------------------------------------------------------------
// Dummy Bitcoin transaction builder stub for unit tests.
// -----------------------------------------------------------------------------

struct NoopTxBuilder;

impl<'info> anchor_lang::context::BtcTxBuilderAny<'info> for NoopTxBuilder {
    fn add_state_transition(
        &mut self,
        _account: &arch_program::account::AccountInfo<'info>,
    ) -> core::result::Result<(), arch_program::program_error::ProgramError> {
        Ok(())
    }
}

/// Build a minimal `BtcContext` suitable for the new `TryFromUtxos` API.
pub fn ctx_for<'info, T>(
    accounts: &'info mut T,
) -> anchor_lang::context::BtcContext<'_, '_, '_, 'info, T>
where
    T: Bumps + anchor_lang::Accounts<'info, T::Bumps>,
    T::Bumps: Default,
{
    // Leak a zero-initialised `Pubkey` so we can obtain a reference with `'static` lifetime.
    let program_id: &'static Pubkey = Box::leak(Box::new(Pubkey::default()));

    // Prepare a no-op transaction builder and leak it so the reference lives long enough.
    let builder: &'static mut NoopTxBuilder = Box::leak(Box::new(NoopTxBuilder));

    anchor_lang::context::BtcContext::new(program_id, accounts, &[], Default::default(), builder)
}
