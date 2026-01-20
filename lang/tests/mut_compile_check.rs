//! Compile-time test to verify that accounts without `#[account(mut)]`
//! cannot be mutated at compile time.
//!
//! This test file contains code that SHOULD compile (with mut) and code
//! that SHOULD NOT compile (without mut). The failing cases are commented
//! out and marked with `// COMPILE_ERROR:` to indicate they should fail.

#![allow(dead_code)]

use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[account]
pub struct TestData {
    pub value: u64,
}

// ============================================================================
// THESE SHOULD COMPILE - accounts have #[account(mut)]
// ============================================================================

#[derive(Accounts)]
pub struct WithMut<'info> {
    #[account(mut)]
    pub data: Account<'info, TestData>,
}

#[program]
pub mod should_compile {
    use super::*;

    pub fn mutate_with_mut(ctx: Context<WithMut>, new_value: u64) -> Result<()> {
        // This SHOULD compile - account has #[account(mut)]
        ctx.accounts.data.value = new_value;
        Ok(())
    }

    pub fn mutate_with_mut_deref(ctx: Context<WithMut>, new_value: u64) -> Result<()> {
        // This SHOULD compile - account has #[account(mut)]
        let data = &mut ctx.accounts.data;
        data.value = new_value;
        Ok(())
    }

    pub fn mutate_with_mut_set_inner(ctx: Context<WithMut>, new_value: u64) -> Result<()> {
        // This SHOULD compile - account has #[account(mut)]
        ctx.accounts.data.set_inner(TestData { value: new_value });
        Ok(())
    }
}

// ============================================================================
// THESE SHOULD NOT COMPILE - accounts do NOT have #[account(mut)]
// ============================================================================

#[derive(Accounts)]
pub struct WithoutMut<'info> {
    // Note: NO #[account(mut)] here
    pub data: Account<'info, TestData>,
}

// Uncomment the function below to verify it fails to compile:
/*
#[program]
pub mod should_not_compile {
    use super::*;

    pub fn mutate_without_mut(ctx: Context<WithoutMut>, new_value: u64) -> Result<()> {
        // COMPILE_ERROR: This should fail - account does NOT have #[account(mut)]
        ctx.accounts.data.value = new_value;
        Ok(())
    }

    pub fn mutate_without_mut_deref(ctx: Context<WithoutMut>, new_value: u64) -> Result<()> {
        // COMPILE_ERROR: This should fail - account does NOT have #[account(mut)]
        let data = &mut ctx.accounts.data;
        data.value = new_value;
        Ok(())
    }
}
*/
