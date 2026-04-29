extern crate alloc;

use {alloc::vec::Vec, anchor_lang_v2::prelude::*};

declare_id!("DupRea11oc1111111111111111111111111111111111");

#[program]
pub mod dup_realloc {
    use super::*;

    #[discrim = 0]
    pub fn init(ctx: &mut Context<Init>) -> Result<()> {
        ctx.accounts.sample.data = alloc::vec![0];
        ctx.accounts.sample.bump = ctx.bumps.sample;
        Ok(())
    }

    #[discrim = 1]
    pub fn realloc_aliased(_ctx: &mut Context<ReallocAliased>, _len: u16) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Init {
    #[account(mut)]
    pub authority: Signer,
    #[account(
        init,
        payer = authority,
        space = 8 + 4 + 1 + 1,
        seeds = [b"sample"],
        bump,
    )]
    pub sample: BorshAccount<Sample>,
    pub system_program: Program<System>,
}

#[derive(Accounts)]
#[instruction(len: u16)]
pub struct ReallocAliased {
    #[account(mut)]
    pub authority: Signer,

    #[account(
        mut,
        seeds = [b"sample"],
        bump = sample1.bump,
        realloc = 8 + 4 + len as usize + 1,
        realloc_payer = authority,
        realloc_zero = false,
        unsafe(dup),
    )]
    pub sample1: BorshAccount<Sample>,

    #[account(
        mut,
        seeds = [b"sample"],
        bump = sample2.bump,
        realloc = 8 + 4 + (len as usize + 10) + 1,
        realloc_payer = authority,
        realloc_zero = false,
        unsafe(dup),
    )]
    pub sample2: BorshAccount<Sample>,

    pub system_program: Program<System>,
}

#[account(borsh)]
pub struct Sample {
    pub data: Vec<u8>,
    pub bump: u8,
}
