//! counter is an example program that depends on an external interface
//! that another program (here counter-auth/src/lib.rs) must implement. This allows
//! our program to depend on another program, without knowing anything about it
//! other than that it implements the `Auth` trait.
//!
//! Here, we have a counter, where, in order to set the count, the `Auth`
//! program must first approve the transaction.

use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod counter {
    use super::*;

    #[state]
    pub struct Counter {
        pub count: u64,
        pub auth_program: Pubkey,
    }

    impl Counter {
        pub fn new(_ctx: Context<Empty>, auth_program: Pubkey) -> Result<Self> {
            Ok(Self {
                count: 0,
                auth_program,
            })
        }

        #[access_control(SetCount::accounts(&self, &ctx))]
        pub fn set_count(&mut self, ctx: Context<SetCount>, new_count: u64) -> Result<()> {
            // Ask the auth program if we should approve the transaction.
            let cpi_program = ctx.accounts.auth_program.clone();
            let cpi_ctx = CpiContext::new(cpi_program, Empty {});
            auth::is_authorized(cpi_ctx, self.count, new_count)?;

            // Approved, so update.
            self.count = new_count;
            Ok(())
        }
    }
}

#[derive(Accounts)]
pub struct Empty {}

#[derive(Accounts)]
pub struct SetCount<'info> {
    auth_program: AccountInfo<'info>,
}

impl<'info> SetCount<'info> {
    // Auxiliary account validation requiring program inputs. As a convention,
    // we separate it from the business logic of the instruction handler itself.
    pub fn accounts(counter: &Counter, ctx: &Context<SetCount>) -> Result<()> {
        if ctx.accounts.auth_program.key != &counter.auth_program {
            return err!(ErrorCode::InvalidAuthProgram);
        }
        Ok(())
    }
}

#[interface]
pub trait Auth<'info, T: Accounts<'info>> {
    fn is_authorized(ctx: Context<T>, current: u64, new: u64) -> Result<()>;
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid auth program.")]
    InvalidAuthProgram,
}
