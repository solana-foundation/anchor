use pinocchio::Resize;

use crate::pinocchio_runtime::account_view::AccountView;
use crate::pinocchio_runtime::system_program;
use crate::prelude::{Id, System};
use crate::Result;

pub fn close(mut info: AccountView, mut sol_destination: AccountView) -> Result<()> {
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
    let _ = Resize::resize(&mut info, 0);
    Ok(())
}

pub fn is_closed(info: &AccountView) -> bool {
    info.owned_by(&System::id()) && info.is_data_empty()
}
