// #region code
use anchor_lang::prelude::*;

#[program]
pub mod basic_4 {
    use super::*;

    #[state]
    pub struct Counter {
        pub authority: Pubkey,
        pub count: u64,
    }

    impl Counter {
        pub fn new(ctx: Context<Auth>) -> Result<Self> {
            Ok(Self {
                authority: *ctx.accounts.authority.key,
                count: 0,
            })
        }

        pub fn increment(&mut self, ctx: Context<Auth>) -> Result<()> {
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
