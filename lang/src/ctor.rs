use crate::{Accounts, ToAccountInfo};
use solana_program::account_info::AccountInfo;

/// The Ctor accounts that can be used to create any account within the program
/// itself (instead of creating the account on the client).
///
/// This is used to create accounts at deterministic addresses, as a function of
/// nothing but a program ID--for example, to create state  global program
/// structs and program IDL accounts. It's currently used **internally** within
/// the Anchor `#[program]` codegen.
#[derive(Accounts)]
pub struct Ctor<'info> {
    // Payer of the transaction.
    #[account(signer)]
    pub from: AccountInfo<'info>,
    // The deterministically defined "state" account being created via
    // `create_account_with_seed`.
    #[account(mut)]
    pub to: AccountInfo<'info>,
    // The program-derived-address signing off on the account creation.
    // Seeds = &[] + bump seed.
    pub base: AccountInfo<'info>,
    // The system program.
    pub system_program: AccountInfo<'info>,
    // The program whose state is being constructed.
    pub program: AccountInfo<'info>,
}
