use anchor_lang::prelude::*;

declare_id!("HmbTLCmaGvZhKnn1Zfa1JVnp7vkMV4DYVxPLWBVoN65L");

#[program]
pub mod misc2 {
    use super::*;

    #[state]
    pub struct MyState {
        pub data: u64,
        pub auth: Pubkey,
    }

    impl MyState {
        pub fn new(ctx: Context<Auth>) -> Result<Self, ProgramError> {
            Ok(Self {
                data: 0,
                auth: *ctx.accounts.authority.key,
            })
        }

        pub fn set_data(&mut self, ctx: Context<Auth>, data: u64) -> Result<(), ProgramError> {
            if self.auth != *ctx.accounts.authority.key {
                return Err(ProgramError::Custom(1234)); // Arbitrary error code.
            }
            self.data = data;
            Ok(())
        }
    }
}

#[derive(Accounts)]
pub struct Auth<'info> {
    #[account(signer)]
    pub authority: AccountInfo<'info>,
}
