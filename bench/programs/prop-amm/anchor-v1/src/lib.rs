use anchor_lang::prelude::*;

declare_id!("55555555555555555555555555555555555555555555");

#[program]
pub mod prop_amm_v1 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let oracle = &mut ctx.accounts.oracle;
        oracle.authority = *ctx.accounts.payer.key;
        oracle.price = 0;
        Ok(())
    }

    pub fn update(ctx: Context<Update>, new_price: u64) -> Result<()> {
        ctx.accounts.oracle.price = new_price;
        Ok(())
    }

    pub fn rotate_authority(
        ctx: Context<RotateAuthority>,
        new_authority: Pubkey,
    ) -> Result<()> {
        ctx.accounts.oracle.authority = new_authority;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(init, payer = payer, space = 8 + 32 + 8)]
    pub oracle: Account<'info, Oracle>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Update<'info> {
    #[account(mut, has_one = authority)]
    pub oracle: Account<'info, Oracle>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct RotateAuthority<'info> {
    #[account(mut, has_one = authority)]
    pub oracle: Account<'info, Oracle>,
    pub authority: Signer<'info>,
}

#[account]
pub struct Oracle {
    pub authority: Pubkey,
    pub price: u64,
}
