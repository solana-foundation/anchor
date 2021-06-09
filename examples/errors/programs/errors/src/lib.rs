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

    pub fn belongs_to_error(_ctx: Context<BelongsToError>) -> Result<()> {
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
pub struct BelongsToError<'info> {
    #[account(init, belongs_to = owner)]
    my_account: ProgramAccount<'info, BelongsToAccount>,
    owner: AccountInfo<'info>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct SignerError<'info> {
    #[account(signer)]
    my_account: AccountInfo<'info>,
}

#[account]
pub struct BelongsToAccount {
    owner: Pubkey,
}

#[error]
pub enum MyError {
    #[msg("This is an error message clients will automatically display")]
    Hello,
    HelloNoMsg = 123,
    HelloNext,
}
