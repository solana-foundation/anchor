#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

// All handlers share: (a: u64, b: u32, c: u64, d: u8)
// Each struct tests a different #[instruction] subset.
// Value constraints prove the deserialized values are correct.
// msg! logs let the test verify the handler received the right values.

#[program]
pub mod test_instruction_validation {
    use super::*;

    pub fn prefix_ab(_ctx: Context<PrefixAB>, _a: u64, _b: u32, _c: u64, _d: u8) -> Result<()> {
        msg!("prefix_ab: a={}, b={}, c={}, d={}", _a, _b, _c, _d);
        Ok(())
    }

    pub fn suffix_cd(_ctx: Context<SuffixCD>, _a: u64, _b: u32, _c: u64, _d: u8) -> Result<()> {
        msg!("suffix_cd: a={}, b={}, c={}, d={}", _a, _b, _c, _d);
        Ok(())
    }

    pub fn single_b(_ctx: Context<SingleB>, _a: u64, _b: u32, _c: u64, _d: u8) -> Result<()> {
        msg!("single_b: a={}, b={}, c={}, d={}", _a, _b, _c, _d);
        Ok(())
    }

    pub fn single_d(_ctx: Context<SingleD>, _a: u64, _b: u32, _c: u64, _d: u8) -> Result<()> {
        msg!("single_d: a={}, b={}, c={}, d={}", _a, _b, _c, _d);
        Ok(())
    }

    pub fn with_seeds(_ctx: Context<WithSeeds>, _a: u64, _b: u32, _c: u64, _d: u8) -> Result<()> {
        msg!("with_seeds: a={}, b={}, c={}, d={}", _a, _b, _c, _d);
        Ok(())
    }

    pub fn hybrid_positional(_ctx: Context<HybridPositional>, _a: u64, _b: u32, _c: u64, _d: u8) -> Result<()> {
        msg!("hybrid_positional: a={}, b={}, c={}, d={}", _a, _b, _c, _d);
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(a: u64, b: u32)]
pub struct PrefixAB<'info> {
    #[account(mut, constraint = a == 100 && b == 200)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(c: u64, d: u8)]
pub struct SuffixCD<'info> {
    #[account(mut, constraint = c == 300 && d == 50)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(b: u32)]
pub struct SingleB<'info> {
    #[account(mut, constraint = b == 200)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(b: u32, d: u8)]
pub struct SingleD<'info> {
    #[account(mut, constraint = b == 200 && d == 50)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(b: u32)]
pub struct WithSeeds<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: PDA derivation is verified by the seeds constraint
    #[account(
        seeds = [b"seed_b", &b.to_le_bytes()],
        bump,
    )]
    pub pda: UncheckedAccount<'info>,
}
#[derive(Accounts)]
#[instruction(x: u64, y: u32)]
pub struct HybridPositional<'info> {
    #[account(mut, constraint = x == 100 && y == 200)]
    pub user: Signer<'info>,
}
