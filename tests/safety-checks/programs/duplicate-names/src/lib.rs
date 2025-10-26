use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod duplicate_names {
    use super::*;
    pub fn func_one(ctx: Context<FuncOne>) -> Result<()> {
        Ok(())
    }
    
    pub fn func_two(ctx: Context<FuncTwo>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct FuncOne<'info> {
    my_account: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct FuncTwo<'info> {
    my_account: UncheckedAccount<'info>,
}
