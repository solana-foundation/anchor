//! This example demonstrates how custom errors and associated error messsages
//! can be defined and transparently propagated to clients.

use anchor_lang::prelude::*;

#[program]
mod errors {
    use super::*;

    pub fn hello(_ctx: Context<Hello>) -> Result<()> {
        Err(MyError::Hello.into())
    }

    pub fn hello_no_msg(_ctx: Context<Hello>) -> Result<()> {
        Err(MyError::HelloNoMsg.into())
    }

    pub fn hello_next(_ctx: Context<Hello>) -> Result<()> {
        Err(MyError::HelloNext.into())
    }

    pub fn mut_error(_ctx: Context<MutError>) -> Result<()> {
        Ok(())
    }

    pub fn has_one_error(_ctx: Context<HasOneError>) -> Result<()> {
        Ok(())
    }

    pub fn signer_error(_ctx: Context<SignerError>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Hello {}

#[derive(Accounts)]
pub struct MutError<'info> {
    #[account(mut)]
    my_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct HasOneError<'info> {
    #[account(init, has_one = owner)]
    my_account: ProgramAccount<'info, HasOneAccount>,
    owner: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SignerError<'info> {
    #[account(signer)]
    my_account: AccountInfo<'info>,
}

#[account]
pub struct HasOneAccount {
    owner: Pubkey,
}

#[error]
pub enum MyError {
    #[msg("This is an error message clients will automatically display")]
    Hello,
    HelloNoMsg = 123,
    HelloNext,
}
