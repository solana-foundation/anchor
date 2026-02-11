#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod test_instruction_validation {
    use super::*;

    pub fn partial_args1(
        _ctx: Context<PartialArgs1>,
        a: u64,
        b: u32,
        c: u64,
        d: u8,
    ) -> Result<()> {
        msg!("a: {}, b: {}, c: {}, d: {}", a, b, c, d);
        Ok(())
    }

    pub fn partial_args2(
        _ctx: Context<PartialArgs2>,
        a: u64,
        b: u32,
        c: u64,
        d: u8,
    ) -> Result<()> {
        msg!("b: {}, d: {}", b, d);
        Ok(())
    }

    pub fn partial_args3(
        _ctx: Context<PartialArgs3>,
        a: u64,
        b: u32,
        c: u64,
        d: u8,
    ) -> Result<()> {
        msg!("a: {}, b: {}", a, b);
        Ok(())
    }

    pub fn partial_args4(
        _ctx: Context<PartialArgs4>,
        a: u64,
        b: u32,
        c: u64,
        d: u8,
    ) -> Result<()> {
        msg!("c: {}, d: {}, a: {}", c, d, a);
        Ok(())
    }

    pub fn partial_args5(
        _ctx: Context<PartialArgs5>,
        a: u64,
        b: u32,
        c: u64,
        d: u8,
    ) -> Result<()> {
        msg!("b: {}", b);
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(d: u8, b: u32)] 
pub struct PartialArgs1<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(b: u32, d: u8)] 
pub struct PartialArgs2<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
}


#[derive(Accounts)]
#[instruction(a: u64, b: u32)] 
pub struct PartialArgs3<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
}
#[derive(Accounts)]
#[instruction(c: u64, d: u8, a: u64)] 
pub struct PartialArgs4<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
}
#[derive(Accounts)]
#[instruction(b: u32)] 
pub struct PartialArgs5<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
}
