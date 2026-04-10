use anchor_lang::prelude::*;
mod pc;
use pc::Price;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod pyth {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, price: i64, expo: i32, conf: u64) -> Result<()> {
        let oracle = &mut ctx.accounts.price;

        let mut price_oracle = Price::load(oracle).unwrap();

        price_oracle.agg.price = price;
        price_oracle.agg.conf = conf;
        price_oracle.expo = expo;
        price_oracle.ptype = pc::PriceType::Price;
        Ok(())
    }

    pub fn set_price(ctx: Context<SetPrice>, price: i64) -> Result<()> {
        let oracle = &mut ctx.accounts.price;
        let mut price_oracle = Price::load(oracle).unwrap();
        price_oracle.agg.price = price as i64;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct SetPrice {
    #[account(mut)]
    pub price: AccountInfo,
}

#[derive(Accounts)]
pub struct Initialize {
    #[account(mut)]
    pub price: AccountInfo,
}
