use {
    crate::{
        pinocchio_runtime::{account_info::AccountInfo, system_program},
        prelude::{Id, System},
        Result,
    },
    pinocchio::Resize,
};

pub fn close(mut info: AccountInfo, mut sol_destination: AccountInfo) -> Result<()> {
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

pub fn is_closed(info: &AccountInfo) -> bool {
    info.owned_by(&System::id()) && info.is_data_empty()
}
