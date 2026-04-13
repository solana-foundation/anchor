use {
    crate::{context::{Bumps, Context}, cursor::AccountCursor},
    pinocchio::address::Address,
    solana_program_error::ProgramError,
};

/// Trait that `#[derive(Accounts)]` implements on account structs.
///
/// Provides the deserialization + constraint-checking entry point
/// (`try_accounts`) and the serialization exit point (`exit_accounts`).
pub trait TryAccounts: Bumps + Sized {
    const HEADER_SIZE: usize;

    fn try_accounts(
        program_id: &Address,
        cursor: &mut AccountCursor,
        ix_data: &[u8],
    ) -> Result<(Self, Self::Bumps), ProgramError>;

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
/// Common scaffold: walks declared accounts via `T::try_accounts`,
/// builds `Context` (with a residual cursor reference for lazy
/// remaining-accounts access), calls the user handler, flushes dirty
/// accounts.
///
/// `num_accounts` is the runtime-observed total account count for this
/// instruction, used to size the remaining-accounts region that
/// `Context::remaining_accounts()` will lazily walk through the same
/// cursor when requested.
#[inline(always)]
pub fn run_handler<'a, T: TryAccounts>(
    program_id: &'a Address,
    cursor: &'a mut AccountCursor,
    ix_data: &[u8],
    num_accounts: usize,
    handler: impl FnOnce(&mut Context<'a, T>) -> Result<(), ProgramError>,
) -> Result<(), ProgramError> {
    if num_accounts < T::HEADER_SIZE {
        return Err(crate::ErrorCode::AccountNotEnoughKeys.into());
    }
    let (ctx_accounts, bumps) = T::try_accounts(program_id, cursor, ix_data)?;
    let remaining_num = (num_accounts - T::HEADER_SIZE) as u8;
    let mut ctx = Context::new(program_id, ctx_accounts, bumps, cursor, remaining_num);
    handler(&mut ctx)?;
    ctx.accounts.exit_accounts()?;
    Ok(())
}
