use crate::pinocchio_runtime::account_info::AccountInfo;
use crate::pinocchio_runtime::system_program;
use crate::prelude::{Id, System};
use crate::Result;

pub fn close(info: AccountInfo, sol_destination: AccountInfo) -> Result<()> {
    // Transfer tokens from the account to the sol_destination.
    let new_dest_lamports = sol_destination
        .lamports()
        .checked_add(info.lamports())
        .ok_or(crate::pinocchio_runtime::program_error::ProgramError::ArithmeticOverflow)?;
    sol_destination.set_lamports(new_dest_lamports);
    info.set_lamports(0);

    unsafe {
        info.assign(&system_program::ID);
    }
    info.resize(0)?;
    Ok(())
}

pub fn is_closed(info: &AccountInfo) -> bool {
    info.owned_by(&System::id()) && info.is_data_empty()
}
