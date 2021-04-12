//! Misc example is a catchall program for testing unrelated features.
//! It's not too instructive/coherent by itself, so please see other examples.

use anchor_lang::prelude::*;
use misc2::misc2::MyState;
use misc2::Auth;

#[program]
pub mod misc {
    use super::*;

    pub const SIZE: u64 = 99;

    #[state(SIZE)]
    pub struct MyState {
        pub v: Vec<u8>,
    }

    impl MyState {
        pub fn new(_ctx: Context<Ctor>) -> Result<Self, ProgramError> {
            Ok(Self { v: vec![] })
        }
    }

    pub fn initialize(ctx: Context<Initialize>, udata: u128, idata: i128) -> ProgramResult {
        ctx.accounts.data.udata = udata;
        ctx.accounts.data.idata = idata;
        Ok(())
    }

    pub fn test_owner(_ctx: Context<TestOwner>) -> ProgramResult {
        Ok(())
    }

    pub fn test_executable(_ctx: Context<TestExecutable>) -> ProgramResult {
        Ok(())
    }

    pub fn test_state_cpi(ctx: Context<TestStateCpi>, data: u64) -> ProgramResult {
        let cpi_program = ctx.accounts.misc2_program.clone();
        let cpi_accounts = Auth {
            authority: ctx.accounts.authority.clone(),
        };
        let ctx = ctx.accounts.cpi_state.context(cpi_program, cpi_accounts);
        misc2::cpi::state::set_data(ctx, data)
    }
}

#[derive(Accounts)]
pub struct Ctor {}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init)]
    data: ProgramAccount<'info, Data>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct TestOwner<'info> {
    #[account(owner = misc)]
    data: AccountInfo<'info>,
    misc: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestExecutable<'info> {
    #[account(executable)]
    program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestStateCpi<'info> {
    #[account(signer)]
    authority: AccountInfo<'info>,
    #[account(mut, state = misc2_program)]
    cpi_state: CpiState<'info, MyState>,
    #[account(executable)]
    misc2_program: AccountInfo<'info>,
}

#[account]
pub struct Data {
    udata: u128,
    idata: i128,
}
