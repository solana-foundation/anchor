use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod realloc {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.sample.data = vec![0];
        ctx.accounts.sample.bump = *ctx.bumps.get("sample").unwrap();
        Ok(())
    }

    pub fn realloc(ctx: Context<Realloc>, len: u8) -> Result<()> {
        ctx.accounts
            .sample
            .data
            .resize_with(len as usize, Default::default);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        seeds = [b"sample"],
        bump,
        space = Sample::space(1),
    )]
    pub sample: Account<'info, Sample>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(len: u8)]
pub struct Realloc<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"sample"],
        bump = sample.bump,
        realloc = Sample::space(len as usize),
        realloc::payer = authority,
        realloc::zero = false,
    )]
    pub sample: Account<'info, Sample>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct Sample {
    pub data: Vec<u8>,
    pub bump: u8,
}

impl Sample {
    pub fn space(len: usize) -> usize {
        8 + (4 + len) + 1
    }
}
