#![no_std]

use quasar_lang::prelude::*;

declare_id!("B7ihZyoXZ1fwAY3TugkiFJ6SXkzJwMuQrxrekBaSmn32");

#[program]
mod hello_world_quasar {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn init(ctx: Ctx<Init>) -> Result<(), ProgramError> {
        let counter = &mut ctx.accounts.counter;
        counter.value = 42u64.into();
        counter.bump = ctx.bumps.counter;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Init<'info> {
    #[account(mut)]
    pub payer: &'info mut Signer,
    #[account(
        init,
        payer = payer,
        seeds = Counter::seeds(),
        bump,
    )]
    pub counter: &'info mut Account<Counter>,
    pub system_program: &'info Program<System>,
}

#[account(discriminator = 1)]
#[seeds(b"counter")]
pub struct Counter {
    pub value: u64,
    pub bump: u8,
    pub _pad: [u8; 7],
}
