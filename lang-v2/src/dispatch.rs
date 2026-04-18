use {
    crate::{
        context::{Bumps, Context},
        cursor::{AccountBitvec, AccountCursor},
        loader::AccountLoader,
    },
    pinocchio::{account::AccountView, address::Address},
    solana_program_error::ProgramError,
};

/// Trait that `#[derive(Accounts)]` implements on account structs.
///
/// `try_accounts` receives a pre-walked `&[AccountView]` slice (from a
/// single `walk_n(HEADER_SIZE)` in [`run_handler`]) rather than the raw
/// cursor.  This lets `Nested<Inner>` fields pass a sub-slice to
/// `Inner::try_accounts` without re-walking the cursor or fighting
/// borrow-checker splits.
///
/// `HEADER_SIZE` is computed recursively at compile time: 1 per direct
/// field, `+ <Inner as TryAccounts>::HEADER_SIZE` per `Nested<Inner>`.
pub trait TryAccounts: Bumps + Sized {
    const HEADER_SIZE: usize;

    /// `base_offset` is the index of the first view in the global bitvec.
    /// Top-level callers pass 0; `Nested<T>` passes its field's offset so
    /// the inner struct's duplicate-mutable-account checks hit the correct
    /// global bits.
    fn try_accounts(
        program_id: &Address,
        views: &[AccountView],
        duplicates: Option<&AccountBitvec>,
        base_offset: usize,
        ix_data: &[u8],
    ) -> Result<(Self, Self::Bumps), ProgramError>;

    fn exit_accounts(&mut self) -> Result<(), ProgramError>;
}

/// Run a handler inside a fully-constructed [`Context`].
///
/// Walks all declared accounts in one `walk_n(HEADER_SIZE)` call, then
/// passes the views slice to `T::try_accounts` for per-field loading and
/// constraint checking.  The residual cursor (past the declared accounts)
/// is handed to `Context` for lazy `remaining_accounts()` access.
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
    let (ctx_accounts, bumps) = {
        let mut loader = AccountLoader::new(program_id, cursor);
        let (views, duplicates) = loader.walk_n(T::HEADER_SIZE);
        T::try_accounts(program_id, views, duplicates, 0, ix_data)?
    };
    let remaining_num = (num_accounts - T::HEADER_SIZE) as u8;
    let mut ctx = Context::new(program_id, ctx_accounts, bumps, cursor, remaining_num);
    handler(&mut ctx)?;
    ctx.accounts.exit_accounts()?;
    Ok(())
}
