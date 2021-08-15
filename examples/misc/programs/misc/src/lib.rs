//! Misc example is a catchall program for testing unrelated features.
//! It's not too instructive/coherent by itself, so please see other examples.

use anchor_lang::prelude::*;
use context::*;
use event::*;
use misc2::Auth;

mod account;
mod context;
mod event;

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

    pub fn test_token_seeds_init(
        _ctx: Context<TestTokenSeedsInit>,
        _token_bump: u8,
        _mint_bump: u8,
    ) -> ProgramResult {
        Ok(())
    }

    pub fn default<'info>(
        _program_id: &Pubkey,
        _accounts: &[AccountInfo<'info>],
        _data: &[u8],
    ) -> ProgramResult {
        Err(ProgramError::Custom(1234))
    }
}
