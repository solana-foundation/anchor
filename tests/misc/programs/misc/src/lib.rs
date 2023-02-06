//! Misc example is a catchall program for testing unrelated features.
//! It's not too instructive/coherent by itself, so please see other examples.

use account::MAX_SIZE;
use anchor_lang::prelude::*;
use context::*;
use event::*;

mod account;
mod context;
mod event;

declare_id!("3TEqcc8xhrhdspwbvoamUJe2borm4Nr72JxL66k6rgrh");

#[constant]
pub const BASE: u128 = 1_000_000;
#[constant]
pub const DECIMALS: u8 = 6;
#[constant]
pub const BYTES_STR: &[u8] = b"test";
#[constant]
pub const BYTE_STR: u8 = b't';
pub const NO_IDL: u16 = 55;

#[program]
pub mod misc {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, udata: u128, idata: i128) -> Result<()> {
        ctx.accounts.data.udata = udata;
        ctx.accounts.data.idata = idata;
        Ok(())
    }

    pub fn initialize_no_rent_exempt(_ctx: Context<InitializeNoRentExempt>) -> Result<()> {
        Ok(())
    }

    pub fn initialize_skip_rent_exempt(_ctx: Context<InitializeSkipRentExempt>) -> Result<()> {
        Ok(())
    }

    pub fn test_owner(_ctx: Context<TestOwner>) -> Result<()> {
        Ok(())
    }

    pub fn test_executable(_ctx: Context<TestExecutable>) -> Result<()> {
        Ok(())
    }

    pub fn test_u16(ctx: Context<TestU16>, data: u16) -> Result<()> {
        ctx.accounts.my_account.data = data;
        Ok(())
    }

    pub fn test_simulate(_ctx: Context<TestSimulate>, data: u32) -> Result<()> {
        emit!(E1 { data });
        emit!(E2 { data: 1234 });
        emit!(E3 { data: 9 });
        emit!(E5 {
            data: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        });
        emit!(E6 {
            data: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
        });
        Ok(())
    }

    pub fn test_input_enum(_ctx: Context<TestSimulate>, data: TestEnum) -> Result<()> {
        emit!(E7 { data: data });
        Ok(())
    }

    pub fn test_i8(ctx: Context<TestI8>, data: i8) -> Result<()> {
        ctx.accounts.data.data = data;
        Ok(())
    }

    pub fn test_i16(ctx: Context<TestI16>, data: i16) -> Result<()> {
        ctx.accounts.data.data = data;
        Ok(())
    }

    pub fn test_const_array_size(ctx: Context<TestConstArraySize>, data: u8) -> Result<()> {
        ctx.accounts.data.data[0] = data;
        Ok(())
    }

    pub fn test_const_ix_data_size(
        ctx: Context<TestConstIxDataSize>,
        data: [u8; MAX_SIZE],
    ) -> Result<()> {
        ctx.accounts.data.data = data;
        Ok(())
    }

    pub fn test_close(_ctx: Context<TestClose>) -> Result<()> {
        Ok(())
    }

    pub fn test_close_twice(ctx: Context<TestCloseTwice>) -> Result<()> {
        let data_account = &ctx.accounts.data;
        let sol_dest_info = ctx.accounts.sol_dest.to_account_info();
        data_account.close(sol_dest_info)?;
        let data_account_info: &AccountInfo = data_account.as_ref();
        require_keys_eq!(*data_account_info.owner, System::id());
        Ok(())
    }

    pub fn test_close_mut(ctx: Context<TestCloseMut>) -> Result<()> {
        let data_account = &ctx.accounts.data;
        let sol_dest_info = ctx.accounts.sol_dest.to_account_info();
        data_account.close(sol_dest_info)?;
        let data_account_info: &AccountInfo = data_account.as_ref();
        require_keys_eq!(*data_account_info.owner, System::id());
        Ok(())
    }

    pub fn test_instruction_constraint(
        _ctx: Context<TestInstructionConstraint>,
        _nonce: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn test_pda_init(
        ctx: Context<TestPdaInit>,
        _domain: String,
        _seed: Vec<u8>,
        _bump: u8,
    ) -> Result<()> {
        ctx.accounts.my_pda.data = 6;
        Ok(())
    }

    pub fn test_pda_init_zero_copy(ctx: Context<TestPdaInitZeroCopy>) -> Result<()> {
        let mut acc = ctx.accounts.my_pda.load_init()?;
        acc.data = 9;
        acc.bump = *ctx.bumps.get("my_pda").unwrap();
        Ok(())
    }

    pub fn test_pda_mut_zero_copy(ctx: Context<TestPdaMutZeroCopy>) -> Result<()> {
        let mut acc = ctx.accounts.my_pda.load_mut()?;
        acc.data = 1234;
        Ok(())
    }

    pub fn test_token_seeds_init(_ctx: Context<TestTokenSeedsInit>) -> Result<()> {
        Ok(())
    }

    pub fn default<'info>(
        _program_id: &Pubkey,
        _accounts: &[AccountInfo<'info>],
        _data: &[u8],
    ) -> Result<()> {
        Err(ProgramError::Custom(1234).into())
    }

    pub fn test_init(ctx: Context<TestInit>) -> Result<()> {
        ctx.accounts.data.data = 3;
        Ok(())
    }

    pub fn test_init_zero_copy(ctx: Context<TestInitZeroCopy>) -> Result<()> {
        let mut data = ctx.accounts.data.load_init()?;
        data.data = 10;
        data.bump = 2;
        Ok(())
    }

    pub fn test_init_mint(ctx: Context<TestInitMint>) -> Result<()> {
        assert!(ctx.accounts.mint.decimals == 6);
        Ok(())
    }

    pub fn test_init_token(ctx: Context<TestInitToken>) -> Result<()> {
        assert!(ctx.accounts.token.mint == ctx.accounts.mint.key());
        Ok(())
    }

    pub fn test_composite_payer(ctx: Context<TestCompositePayer>) -> Result<()> {
        ctx.accounts.composite.data.data = 1;
        ctx.accounts.data.udata = 2;
        ctx.accounts.data.idata = 3;
        Ok(())
    }

    pub fn test_init_associated_token(ctx: Context<TestInitAssociatedToken>) -> Result<()> {
        assert!(ctx.accounts.token.mint == ctx.accounts.mint.key());
        Ok(())
    }

    pub fn test_validate_associated_token(
        _ctx: Context<TestValidateAssociatedToken>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn test_fetch_all(ctx: Context<TestFetchAll>, filterable: Pubkey) -> Result<()> {
        ctx.accounts.data.authority = ctx.accounts.authority.key();
        ctx.accounts.data.filterable = filterable;
        Ok(())
    }

    pub fn test_init_with_empty_seeds(_ctx: Context<TestInitWithEmptySeeds>) -> Result<()> {
        Ok(())
    }

    pub fn test_empty_seeds_constraint(_ctx: Context<TestEmptySeedsConstraint>) -> Result<()> {
        Ok(())
    }

    pub fn test_init_if_needed(ctx: Context<TestInitIfNeeded>, data: u16) -> Result<()> {
        ctx.accounts.data.data = data;
        Ok(())
    }

    pub fn test_init_if_needed_checks_owner(
        _ctx: Context<TestInitIfNeededChecksOwner>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn test_init_if_needed_checks_seeds(
        _ctx: Context<TestInitIfNeededChecksSeeds>,
        _seed_data: String,
    ) -> Result<()> {
        Ok(())
    }

    pub fn test_init_mint_if_needed(
        _ctx: Context<TestInitMintIfNeeded>,
        _decimals: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn test_init_token_if_needed(_ctx: Context<TestInitTokenIfNeeded>) -> Result<()> {
        Ok(())
    }

    pub fn test_init_associated_token_if_needed(
        _ctx: Context<TestInitAssociatedTokenIfNeeded>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn init_with_space(_ctx: Context<InitWithSpace>, _data: u16) -> Result<()> {
        Ok(())
    }

    pub fn test_multidimensional_array(
        ctx: Context<TestMultidimensionalArray>,
        data: [[u8; 10]; 10],
    ) -> Result<()> {
        ctx.accounts.data.data = data;
        Ok(())
    }

    pub fn test_multidimensional_array_const_sizes(
        ctx: Context<TestMultidimensionalArrayConstSizes>,
        data: [[u8; 11]; 10],
    ) -> Result<()> {
        ctx.accounts.data.data = data;
        Ok(())
    }

    pub fn test_no_rent_exempt(_ctx: Context<NoRentExempt>) -> Result<()> {
        Ok(())
    }

    pub fn test_enforce_rent_exempt(_ctx: Context<EnforceRentExempt>) -> Result<()> {
        Ok(())
    }

    pub fn init_decrease_lamports(ctx: Context<InitDecreaseLamports>) -> Result<()> {
        **ctx.accounts.data.try_borrow_mut_lamports()? -= 1;
        **ctx.accounts.user.try_borrow_mut_lamports()? += 1;
        Ok(())
    }

    pub fn init_if_needed_checks_rent_exemption(
        _ctx: Context<InitIfNeededChecksRentExemption>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn test_program_id_constraint(
        _ctx: Context<TestProgramIdConstraint>,
        _bump: u8,
        _second_bump: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn test_program_id_constraint_find_pda(
        _ctx: Context<TestProgramIdConstraintUsingFindPda>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn test_token_constraint(_ctx: Context<TestConstraintToken>) -> Result<()> {
        Ok(())
    }

    pub fn test_token_auth_constraint(_ctx: Context<TestAuthorityConstraint>) -> Result<()> {
        Ok(())
    }

    pub fn test_only_auth_constraint(_ctx: Context<TestOnlyAuthorityConstraint>) -> Result<()> {
        Ok(())
    }

    pub fn test_only_mint_constraint(_ctx: Context<TestOnlyMintConstraint>) -> Result<()> {
        Ok(())
    }

    pub fn test_mint_constraint(_ctx: Context<TestMintConstraint>, _decimals: u8) -> Result<()> {
        Ok(())
    }

    pub fn test_mint_only_decimals_constraint(
        _ctx: Context<TestMintOnlyDecimalsConstraint>,
        _decimals: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn test_mint_only_auth_constraint(
        _ctx: Context<TestMintAuthorityConstraint>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn test_mint_only_one_auth_constraint(
        _ctx: Context<TestMintOneAuthorityConstraint>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn test_mint_miss_mint_auth_constraint(
        _ctx: Context<TestMintMissMintAuthConstraint>,
        _decimals: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn test_associated_constraint(_ctx: Context<TestAssociatedToken>) -> Result<()> {
        Ok(())
    }
}
