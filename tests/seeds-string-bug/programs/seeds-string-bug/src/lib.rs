use anchor_lang::prelude::*;

declare_id!("4sBmrLYW4wdCgmLDj8hxR1i8w8kGWJWYU6nQPBz9oNLX");

#[program]
pub mod seeds_string_bug {
    use super::*;

    pub fn initialize_cohort(
        ctx: Context<InitializeCohort>,
        cohort_name: String,
        sport: String,
        year: u16,
        club_name: String,
        max_supply: u64,
        mint_price: u64,
    ) -> Result<()> {
        let cohort = &mut ctx.accounts.cohort;
        cohort.cohort_name = cohort_name;
        cohort.sport = sport;
        cohort.year = year;
        cohort.club_name = club_name;
        cohort.max_supply = max_supply;
        cohort.mint_price = mint_price;
        cohort.bump = ctx.bumps.cohort;
        Ok(())
    }

    pub fn test_pda_derivation(
        _ctx: Context<TestPdaDerivation>,
        _cohort_name: String,
        _year: u16,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(cohort_name: String, sport: String, year: u16, club_name: String, max_supply: u64, mint_price: u64)]
pub struct InitializeCohort<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Cohort::INIT_SPACE,
        seeds = [b"cohort", cohort_name.as_bytes(), year.to_le_bytes().as_ref()],
        bump
    )]
    pub cohort: Account<'info, Cohort>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(cohort_name: String, year: u16)]
pub struct TestPdaDerivation<'info> {
    #[account(
        seeds = [b"cohort", cohort_name.as_bytes(), year.to_le_bytes().as_ref()],
        bump
    )]
    pub cohort: Account<'info, Cohort>,
}

#[account]
#[derive(InitSpace)]
pub struct Cohort {
    #[max_len(64)]
    pub cohort_name: String,
    #[max_len(32)]
    pub sport: String,
    pub year: u16,
    #[max_len(64)]
    pub club_name: String,
    pub max_supply: u64,
    pub mint_price: u64,
    pub bump: u8,
} 