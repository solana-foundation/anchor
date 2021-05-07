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

    pub fn test_associated_account_creation(
        ctx: Context<TestAssociatedAccount>,
        data: u64,
    ) -> ProgramResult {
        ctx.accounts.my_account.data = data;
        Ok(())
    }

    pub fn test_u16(ctx: Context<TestU16>, data: u16) -> ProgramResult {
        ctx.accounts.my_account.data = data;
        Ok(())
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

// `my_account` is the associated token account being created.
// `authority` must be a `mut` and `signer` since it will pay for the creation
// of the associated token account. `state` is used as an association, i.e., one
// can *optionally* identify targets to be used as seeds for the program
// derived address by using `with` (and it doesn't have to be a state account).
// For example, the SPL token program uses a `Mint` account. Lastly,
// `rent` and `system_program` are *required* by convention, since the
// accounts are needed when creating the associated program address within
// the program.
#[derive(Accounts)]
pub struct TestAssociatedAccount<'info> {
    #[account(associated = authority, with = state, with = data)]
    my_account: ProgramAccount<'info, TestData>,
    #[account(mut, signer)]
    authority: AccountInfo<'info>,
    state: ProgramState<'info, MyState>,
    data: ProgramAccount<'info, Data>,
    rent: Sysvar<'info, Rent>,
    system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestU16<'info> {
    #[account(init)]
    my_account: ProgramAccount<'info, DataU16>,
    rent: Sysvar<'info, Rent>,
}

#[associated]
pub struct TestData {
    data: u64,
}

#[account]
pub struct Data {
    udata: u128,
    idata: i128,
}

#[account]
pub struct DataU16 {
    data: u16,
}
