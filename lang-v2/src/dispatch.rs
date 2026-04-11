use {
    pinocchio::{
        account::AccountView,
        address::Address,
    },
    solana_program_error::ProgramError,
    crate::{context::{Context, Bumps}},
};

/// Trait that `#[derive(Accounts)]` implements on account structs.
///
/// Provides the deserialization + constraint-checking entry point
/// (`try_accounts`) and the serialization exit point (`exit_accounts`).
pub trait TryAccounts: Bumps + Sized {
    fn try_accounts(
        program_id: &Address,
        accounts: &[AccountView],
        ix_data: &[u8],
    ) -> Result<(Self, Self::Bumps, usize), ProgramError>;

    fn exit_accounts(&mut self) -> Result<(), ProgramError>;
}

/// Parse the 8-byte discriminator from instruction data.
///
/// Returns `(discriminator_u64, remaining_ix_data)`.
#[inline(always)]
pub fn parse_instruction(data: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
    if data.len() < 8 {
        return Err(crate::ErrorCode::InstructionFallbackNotFound.into());
    }
    let disc = u64::from_le_bytes(data[..8].try_into().unwrap());
    Ok((disc, &data[8..]))
}

/// Run a handler inside a fully-constructed [`Context`].
///
/// Common scaffold: build context, call user function, flush dirty accounts.
#[inline(always)]
pub fn run_handler<T: TryAccounts>(
    program_id: &Address,
    accounts: &[AccountView],
    ix_data: &[u8],
    handler: impl FnOnce(&mut Context<T>) -> Result<(), ProgramError>,
) -> Result<(), ProgramError> {
    let (ctx_accounts, bumps, consumed) = T::try_accounts(program_id, accounts, ix_data)?;
    // `get(consumed..)` returns `Option`, so LLVM doesn't emit
    // `slice_start_index_len_fail` and doesn't drag in core::fmt panic
    // formatters for the "out of range for slice of length" message.
    let remaining = accounts
        .get(consumed..)
        .ok_or::<ProgramError>(crate::ErrorCode::AccountNotEnoughKeys.into())?;
    let mut ctx = Context::new(*program_id, ctx_accounts, remaining, bumps);
    handler(&mut ctx)?;
    ctx.accounts.exit_accounts()?;
    Ok(())
}
