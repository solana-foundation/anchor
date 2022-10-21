use crate::prelude::{Id, System};
use crate::Result;
use solana_program::account_info::AccountInfo;
use solana_program::system_program;

pub fn close<'info>(info: AccountInfo<'info>, sol_destination: AccountInfo<'info>) -> Result<()> {
    // Transfer tokens from the account to the sol_destination.
    let dest_starting_lamports = sol_destination.lamports();
    **sol_destination.lamports.borrow_mut() =
        dest_starting_lamports.checked_add(info.lamports()).unwrap();
    **info.lamports.borrow_mut() = 0;

    info.assign(&system_program::ID);
    info.realloc(0, false).map_err(Into::into)
}

pub fn is_closed(info: &AccountInfo) -> bool {
    info.owner == &System::id() && info.data_is_empty()
}
