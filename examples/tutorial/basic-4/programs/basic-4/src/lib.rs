#![feature(proc_macro_hygiene)]

// #region code
use anchor_lang::prelude::*;

#[program]
pub mod basic_4 {
    use super::*;

    #[state]
    pub struct Counter {
        authority: Pubkey,
        count: u64,
    }

    impl Counter {
        pub fn new(ctx: Context<Auth>) -> Result<Self, ProgramError> {
            Ok(Self {
                authority: *ctx.accounts.authority.key,
                count: 0,
            })
        }

        pub fn increment(&mut self, ctx: Context<Auth>) -> Result<(), Error> {
            if &self.authority != ctx.accounts.authority.key {
                return Err(ErrorCode::Unauthorized.into());
            }
            self.count += 1;
            Ok(())
        }
    }
}

#[derive(Accounts)]
pub struct Auth<'info> {
    #[account(signer)]
    authority: AccountInfo<'info>,
}
// #endregion code

#[error]
pub enum ErrorCode {
    #[msg("You are not authorized to perform this action.")]
    Unauthorized,
}
