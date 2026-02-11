use crate::prelude::{Id, System};
use crate::solana_program::account_info::AccountInfo;
use crate::solana_program::system_program;
use crate::Result;

pub fn close<'info>(info: AccountInfo<'info>, sol_destination: AccountInfo<'info>) -> Result<()> {
    // Zero out the account discriminator to prevent revival via init_if_needed.
    // Even if the account hasn't been garbage collected yet, the zeroed
    // discriminator ensures Anchor's deserialization will reject it.
    {
        let mut data = info.try_borrow_mut_data()?;
        // Zero the first 8 bytes (Anchor discriminator)
        let len = std::cmp::min(data.len(), 8);
        for byte in data[..len].iter_mut() {
            *byte = 0;
        }
    }

    // Transfer tokens from the account to the sol_destination.
    let dest_starting_lamports = sol_destination.lamports();
    **sol_destination.lamports.borrow_mut() =
        dest_starting_lamports.checked_add(info.lamports()).unwrap();
    **info.lamports.borrow_mut() = 0;

    info.assign(&system_program::ID);
    info.resize(0).map_err(Into::into)
}

pub fn is_closed(info: &AccountInfo) -> bool {
    info.owner == &System::id() && info.data_is_empty()
}
