#![allow(unused_variables)]

use anchor_lang::prelude::*;

declare_id!("Externa111111111111111111111111111111111111");

/// Master seed slice
#[constant]
pub const MASTER_SEED: &[u8] = b"master";

#[program]
pub mod external {
    use super::*;

    pub fn init(_ctx: Context<Init>) -> Result<()> {
        Ok(())
    }

    pub fn update(ctx: Context<Update>, value: u32) -> Result<()> {
        ctx.accounts.my_account.field = value;
        Ok(())
    }

    pub fn update_composite(ctx: Context<UpdateComposite>, value: u32) -> Result<()> {
        ctx.accounts.update.my_account.field = value;
        Ok(())
    }

    // Test the issue described in https://github.com/coral-xyz/anchor/issues/3274
    pub fn update_non_instruction_composite(
        ctx: Context<UpdateNonInstructionComposite>,
        value: u32,
    ) -> Result<()> {
        ctx.accounts.non_instruction_update.my_account.field = value;
        Ok(())
    }

    // Compilation test for whether a defined type (an account in this case) can be used in `cpi` client.
    pub fn test_compilation_defined_type_param(
        _ctx: Context<TestCompilation>,
        _my_account: MyAccount,
    ) -> Result<()> {
        Ok(())
    }

    // Compilation test for whether a custom return type can be specified in `cpi` client
    pub fn test_compilation_return_type(_ctx: Context<TestCompilation>) -> Result<bool> {
        Ok(true)
    }

    // Compilation test for whether `data` can be used as an instruction parameter name
    pub fn test_compilation_data_as_parameter_name(
        _ctx: Context<TestCompilation>,
        data: Vec<u8>,
    ) -> Result<()> {
        Ok(())
    }

    // Compilation test for an instruction with no accounts
    pub fn test_compilation_no_accounts(_ctx: Context<TestCompilationNoAccounts>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct TestCompilation<'info> {
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct TestCompilationNoAccounts {}

#[derive(Accounts)]
pub struct Init<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = 8 + 4,
        seeds = [authority.key.as_ref()],
        bump
    )]
    pub my_account: Account<'info, MyAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Update<'info> {
    pub authority: Signer<'info>,
    #[account(mut, seeds = [authority.key.as_ref()], bump)]
    pub my_account: Account<'info, MyAccount>,
}

#[derive(Accounts)]
pub struct NonInstructionUpdate<'info> {
    pub authority: Signer<'info>,
    #[account(mut, seeds = [authority.key.as_ref()], bump)]
    pub my_account: Account<'info, MyAccount>,
    pub program: Program<'info, program::External>,
}

#[derive(Accounts)]
pub struct UpdateComposite<'info> {
    pub update: Update<'info>,
}

#[derive(Accounts)]
pub struct UpdateNonInstructionComposite<'info> {
    pub non_instruction_update: NonInstructionUpdate<'info>,
}

#[account]
pub struct MyAccount {
    pub field: u32,
}

#[event]
pub struct MyEvent {
    pub value: u32,
}
