//! Misc example is a catchall program for testing unrelated features.
//! It's not too instructive/coherent by itself, so please see other examples.

use anchor_lang::prelude::*;
use misc2::misc2::MyState as Misc2State;
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

        pub fn remaining_accounts(&mut self, ctx: Context<RemainingAccounts>) -> ProgramResult {
            if ctx.remaining_accounts.len() != 1 {
                return Err(ProgramError::Custom(1)); // Arbitrary error.
            }
            Ok(())
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

    pub fn test_init_associated_account(
        ctx: Context<TestInitAssociatedAccount>,
        data: u64,
    ) -> ProgramResult {
        ctx.accounts.my_account.data = data;
        Ok(())
    }

    pub fn test_associated_account(
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

    pub fn test_simulate(_ctx: Context<TestSimulate>, data: u32) -> ProgramResult {
        emit!(E1 { data });
        emit!(E2 { data: 1234 });
        emit!(E3 { data: 9 });
        Ok(())
    }

    pub fn test_i8(ctx: Context<TestI8>, data: i8) -> ProgramResult {
        ctx.accounts.data.data = data;
        Ok(())
    }

    pub fn test_i16(ctx: Context<TestI16>, data: i16) -> ProgramResult {
        ctx.accounts.data.data = data;
        Ok(())
    }

    pub fn test_close(_ctx: Context<TestClose>) -> ProgramResult {
        Ok(())
    }

    pub fn test_instruction_constraint(
        _ctx: Context<TestInstructionConstraint>,
        _nonce: u8,
    ) -> ProgramResult {
        Ok(())
    }

    pub fn test_pda_init(
        ctx: Context<TestPdaInit>,
        _domain: String,
        _seed: Vec<u8>,
        _bump: u8,
    ) -> ProgramResult {
        ctx.accounts.my_pda.data = 6;
        Ok(())
    }

    pub fn test_pda_init_zero_copy(ctx: Context<TestPdaInitZeroCopy>, bump: u8) -> ProgramResult {
        let mut acc = ctx.accounts.my_pda.load_init()?;
        acc.data = 9;
        acc.bump = bump;
        Ok(())
    }

    pub fn test_pda_mut_zero_copy(ctx: Context<TestPdaMutZeroCopy>) -> ProgramResult {
        let mut acc = ctx.accounts.my_pda.load_mut()?;
        acc.data = 1234;
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(nonce: u8)]
pub struct TestInstructionConstraint<'info> {
    #[account(seeds = [b"my-seed", my_account.key.as_ref(), &[nonce]])]
    pub my_pda: AccountInfo<'info>,
    pub my_account: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(domain: String, seed: Vec<u8>, bump: u8)]
pub struct TestPdaInit<'info> {
    #[account(
        init,
        seeds = [b"my-seed", domain.as_bytes(), foo.key.as_ref(), &seed, &[bump]],
        payer = my_payer,
    )]
    my_pda: ProgramAccount<'info, DataU16>,
    my_payer: AccountInfo<'info>,
    foo: AccountInfo<'info>,
    rent: Sysvar<'info, Rent>,
    system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct TestPdaInitZeroCopy<'info> {
    #[account(init, seeds = [b"my-seed".as_ref(), &[bump]], payer = my_payer)]
    my_pda: Loader<'info, DataZeroCopy>,
    my_payer: AccountInfo<'info>,
    rent: Sysvar<'info, Rent>,
    system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestPdaMutZeroCopy<'info> {
    #[account(mut, seeds = [b"my-seed".as_ref(), &[my_pda.load()?.bump]])]
    my_pda: Loader<'info, DataZeroCopy>,
    my_payer: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Ctor {}

#[derive(Accounts)]
pub struct RemainingAccounts {}

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
    cpi_state: CpiState<'info, Misc2State>,
    #[account(executable)]
    misc2_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestClose<'info> {
    #[account(mut, close = sol_dest)]
    data: ProgramAccount<'info, Data>,
    sol_dest: AccountInfo<'info>,
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
pub struct TestInitAssociatedAccount<'info> {
    #[account(init, associated = authority, with = state, with = data)]
    my_account: ProgramAccount<'info, TestData>,
    #[account(mut, signer)]
    authority: AccountInfo<'info>,
    state: ProgramState<'info, MyState>,
    data: ProgramAccount<'info, Data>,
    rent: Sysvar<'info, Rent>,
    system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestAssociatedAccount<'info> {
    #[account(mut, associated = authority, with = state, with = data)]
    my_account: ProgramAccount<'info, TestData>,
    #[account(mut, signer)]
    authority: AccountInfo<'info>,
    state: ProgramState<'info, MyState>,
    data: ProgramAccount<'info, Data>,
}

#[derive(Accounts)]
pub struct TestU16<'info> {
    #[account(init)]
    my_account: ProgramAccount<'info, DataU16>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct TestI16<'info> {
    #[account(init)]
    data: ProgramAccount<'info, DataI16>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct TestSimulate {}

#[derive(Accounts)]
pub struct TestI8<'info> {
    #[account(init)]
    data: ProgramAccount<'info, DataI8>,
    rent: Sysvar<'info, Rent>,
}

#[associated]
#[derive(Default)]
pub struct TestData {
    data: u64,
}

#[account]
pub struct Data {
    udata: u128,
    idata: i128,
}

#[account]
#[derive(Default)]
pub struct DataU16 {
    data: u16,
}

#[account]
pub struct DataI8 {
    data: i8,
}

#[account]
pub struct DataI16 {
    data: i16,
}

#[account(zero_copy)]
#[derive(Default)]
pub struct DataZeroCopy {
    data: u16,
    bump: u8,
}

#[event]
pub struct E1 {
    data: u32,
}

#[event]
pub struct E2 {
    data: u32,
}

#[event]
pub struct E3 {
    data: u32,
}
