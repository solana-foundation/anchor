use crate::bpf_writer::BpfWriter;
use crate::error::ErrorCode;
use crate::prelude::error;
use crate::Result;
use solana_program::account_info::AccountInfo;
use std::io::Write;

pub fn close<'info>(info: AccountInfo<'info>, sol_destination: AccountInfo<'info>) -> Result<()> {
    // Transfer tokens from the account to the sol_destination.
    let dest_starting_lamports = sol_destination.lamports();
    **sol_destination.lamports.borrow_mut() =
        dest_starting_lamports.checked_add(info.lamports()).unwrap();
    **info.lamports.borrow_mut() = 0;

    // Mark the account discriminator as closed.
    let mut data = info.try_borrow_mut_data()?;
    let dst: &mut [u8] = &mut data;
    let mut writer = BpfWriter::new(dst);
    writer
        .write_all(&crate::__private::CLOSED_ACCOUNT_DISCRIMINATOR)
        .map_err(|_| error!(ErrorCode::AccountDidNotSerialize))
}
