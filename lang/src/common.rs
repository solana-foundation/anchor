use crate::pinocchio_runtime::account_info::AccountInfo;
use crate::pinocchio_runtime::system_program;
use crate::prelude::{Id, System};
use crate::Result;

pub fn close<'info>(info: AccountInfo, sol_destination: AccountInfo) -> Result<()> {
    // Transfer tokens from the account to the sol_destination.
    let dest_starting_lamports = sol_destination.lamports();
    let mut dest_lamports = sol_destination.try_borrow_mut_lamports()?;
    *dest_lamports = dest_starting_lamports
        .checked_add(info.lamports())
        .ok_or(crate::pinocchio_runtime::program_error::ProgramError::ArithmeticOverflow)?;
    drop(dest_lamports);

    let mut info_lamports = info.try_borrow_mut_lamports()?;
    *info_lamports = 0;
    drop(info_lamports);

    info.assign(system_program::ID)?;
    info.resize(0).map_err(Into::into)
}

pub fn is_closed(info: &AccountInfo) -> bool {
    info.owner() == &System::id() && info.data_is_empty()
}
