// #region code
use anchor_lang::prelude::*;

declare_id!("CwrqeMj2U8tFr1Rhkgwc84tpAsqbt9pTt2a4taoTADPr");

#[program]
pub mod basic_4 {
    use super::*;

    #[state]
    pub struct Counter {
        pub authority: Pubkey,
        pub count: u64,
    }

    impl Counter {
        pub fn new(ctx: Context<Auth>) -> anchor_lang::Result<Self> {
            Ok(Self {
                authority: *ctx.accounts.authority.key,
                count: 0,
            })
        }

        pub fn increment(&mut self, ctx: Context<Auth>) -> anchor_lang::Result<()> {
            if &self.authority != ctx.accounts.authority.key {
                return Err(error!(ErrorCode::Unauthorized));
            }
            self.count += 1;
            Ok(())
        }
    }
}

#[derive(Accounts)]
pub struct Auth<'info> {
    authority: Signer<'info>,
}
// #endregion code

#[error_code]
pub enum ErrorCode {
    #[msg("You are not authorized to perform this action.")]
    Unauthorized,
}
