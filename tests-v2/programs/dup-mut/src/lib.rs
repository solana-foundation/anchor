//! Test program for the duplicate-mutable-account safety check.
//!
//! Exercises each combination the derive must reject at `try_accounts`
//! time, plus the `unsafe(dup)` escape hatch. The handlers for unsafe
//! variants are written to never hold two `&mut Data` live at once, so
//! invoking them with aliased inputs does not produce UB.

use anchor_lang_v2::prelude::*;

declare_id!("2TxMd2YAMi9Sk4xxiJBNkYQNuxK9FwvwwiujuEbKoanz");

#[program]
pub mod dup_mut {
    use super::*;

    pub fn initialize(_ctx: &mut Context<Initialize>, seed: u8) -> Result<()> {
        let _ = seed;
        Ok(())
    }

    pub fn touch_two_mut(ctx: &mut Context<TouchTwoMut>, value: u64) -> Result<()> {
        ctx.accounts.data_a.value = value;
        ctx.accounts.data_b.value = value.wrapping_add(1);
        Ok(())
    }

    pub fn touch_three_mut(ctx: &mut Context<TouchThreeMut>, value: u64) -> Result<()> {
        ctx.accounts.data_a.value = value;
        ctx.accounts.data_b.value = value.wrapping_add(1);
        ctx.accounts.data_c.value = value.wrapping_add(2);
        Ok(())
    }

    pub fn touch_mut_and_readonly(
        ctx: &mut Context<TouchMutAndReadonly>,
        value: u64,
    ) -> Result<()> {
        ctx.accounts.data_a.value = value;
        Ok(())
    }

    pub fn touch_two_mut_asym_unsafe(
        ctx: &mut Context<TouchTwoMutAsymUnsafe>,
        value: u64,
    ) -> Result<()> {
        // Reachable only with distinct pubkeys: data_a has no `unsafe(dup)`,
        // so an aliased call still trips the generated check on position 0.
        ctx.accounts.data_a.value = value;
        ctx.accounts.data_b.value = value.wrapping_add(1);
        Ok(())
    }

    pub fn touch_two_mut_unsafe(
        ctx: &mut Context<TouchTwoMutUnsafe>,
        value: u64,
    ) -> Result<()> {
        // SAFETY: When invoked with data_a == data_b, both fields alias the
        // same account data. We only ever materialize ONE `&mut Data` (via
        // `data_a`). `data_b` is never deref'd, so no two `&mut` to the same
        // bytes exist simultaneously and no UB is possible.
        ctx.accounts.data_a.value = value;
        let _ = &ctx.accounts.data_b;
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(seed: u8)]
pub struct Initialize {
    #[account(mut)]
    pub payer: Signer,
    #[account(
        init,
        payer = payer,
        space = 8 + core::mem::size_of::<Data>(),
        seeds = [b"d", &[seed]],
        bump,
    )]
    pub data: Account<Data>,
    pub system_program: Program<System>,
}

#[derive(Accounts)]
pub struct TouchTwoMut {
    #[account(mut)]
    pub data_a: Account<Data>,
    #[account(mut)]
    pub data_b: Account<Data>,
}

#[derive(Accounts)]
pub struct TouchThreeMut {
    #[account(mut)]
    pub data_a: Account<Data>,
    #[account(mut)]
    pub data_b: Account<Data>,
    #[account(mut)]
    pub data_c: Account<Data>,
}

#[derive(Accounts)]
pub struct TouchMutAndReadonly {
    #[account(mut)]
    pub data_a: Account<Data>,
    pub data_b: Account<Data>,
}

#[derive(Accounts)]
pub struct TouchTwoMutAsymUnsafe {
    #[account(mut)]
    pub data_a: Account<Data>,
    #[account(mut, unsafe(dup))]
    pub data_b: Account<Data>,
}

#[derive(Accounts)]
pub struct TouchTwoMutUnsafe {
    #[account(mut, unsafe(dup))]
    pub data_a: Account<Data>,
    #[account(mut, unsafe(dup))]
    pub data_b: Account<Data>,
}

#[account]
pub struct Data {
    pub value: u64,
}
