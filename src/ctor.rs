use crate::{Accounts, Sysvar};
use solana_program::account_info::AccountInfo;
use solana_program::sysvar::rent::Rent;

// Needed for the `Accounts` macro.
use crate as anchor_lang;

#[derive(Accounts)]
pub struct Ctor<'info> {
    pub from: AccountInfo<'info>,
    #[account(mut)]
    pub to: AccountInfo<'info>,
    pub base: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub program: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
}
