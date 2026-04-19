use crate::{
    error::ErrorCode,
    prelude::{Id, System},
    solana_program::{account_info::AccountInfo, system_program},
    Result,
};

/// Closes `info` by transferring all of its lamports to `sol_destination` and
/// zeroing out its data and owner.
///
/// # Errors
///
/// Returns [`ErrorCode::ArithmeticOverflow`] if adding `info`'s lamports to
/// `sol_destination`'s existing balance would overflow a `u64`. In practice
/// this can only happen if the combined SOL exceeds `u64::MAX`, which is
/// impossible on mainnet given current total supply.
pub(crate) fn close<'info>(
    info: &AccountInfo<'info>,
    sol_destination: &AccountInfo<'info>,
) -> Result<()> {
    // Transfer all lamports from the closing account to the destination.
    // We use `checked_add` to guard against arithmetic overflow and propagate
    // a graceful error instead of panicking with `unwrap()`.
    let dest_starting_lamports = sol_destination.lamports();
    **sol_destination.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(info.lamports())
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    // Zero out the closing account's lamports so it becomes rent-exempt-clean.
    **info.lamports.borrow_mut() = 0;

    // Reassign ownership to the System Program and clear all data, which is the
    // canonical on-chain representation of a "closed" account.
    info.assign(&system_program::ID);
    info.resize(0).map_err(Into::into)
}

pub fn is_closed(info: &AccountInfo) -> bool {
    info.owner == &System::id() && info.data_is_empty()
}
