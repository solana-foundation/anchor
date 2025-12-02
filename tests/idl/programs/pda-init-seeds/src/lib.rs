use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub const POOL_PREFIX: &[u8] = b"cpool";

#[program]
pub mod pda_init_seeds {
    use super::*;

    pub fn initialize_pool(ctx: Context<InitializePool>) -> Result<()> {
        ctx.accounts.pool.data = 42;
        Ok(())
    }

    pub fn initialize_position(ctx: Context<InitializePosition>) -> Result<()> {
        ctx.accounts.position.data = 100;
        Ok(())
    }

    pub fn initialize_customizable_pool(ctx: Context<InitializeCustomizablePool>) -> Result<()> {
        ctx.accounts.pool.data = 999;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(
        init,
        seeds = [
            b"pool",
            token_a_mint.key().as_ref(),
            token_b_mint.key().as_ref(),
        ],
        bump,
        payer = payer,
        space = 8 + Pool::INIT_SPACE
    )]
    pub pool: Account<'info, Pool>,

    /// CHECK: Token A mint
    pub token_a_mint: AccountInfo<'info>,
    /// CHECK: Token B mint
    pub token_b_mint: AccountInfo<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializePosition<'info> {
    #[account(
        init,
        seeds = [
            b"position",
            position_nft_mint.key().as_ref()
        ],
        bump,
        payer = payer,
        space = 8 + Position::INIT_SPACE
    )]
    pub position: Account<'info, Position>,

    /// CHECK: Position NFT mint
    pub position_nft_mint: AccountInfo<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

// Test case with const prefix from module scope
// This tests init + seeds with const values from outside the struct
#[derive(Accounts)]
pub struct InitializeCustomizablePool<'info> {
    #[account(
        init,
        seeds = [
            POOL_PREFIX,
            token_a_mint.key().as_ref(),
            token_b_mint.key().as_ref(),
        ],
        bump,
        payer = payer,
        space = 8 + CustomizablePool::INIT_SPACE
    )]
    pub pool: Account<'info, CustomizablePool>,

    /// CHECK: Token A mint
    pub token_a_mint: AccountInfo<'info>,
    /// CHECK: Token B mint
    pub token_b_mint: AccountInfo<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct Pool {
    pub data: u64,
}

impl Pool {
    pub const INIT_SPACE: usize = 8; // u64
}

#[account]
pub struct Position {
    pub data: u64,
}

impl Position {
    pub const INIT_SPACE: usize = 8; // u64
}

#[account]
pub struct CustomizablePool {
    pub data: u64,
}

impl CustomizablePool {
    pub const INIT_SPACE: usize = 8; // u64
}
