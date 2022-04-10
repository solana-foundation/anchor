//! The typescript example serves to show how one would setup an Anchor
//! workspace with TypeScript tests and migrations.

use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub const MY_SEED: [u8; 2] = *b"hi";
pub const MY_SEED_STR: &str = "hi";
pub const MY_SEED_U8: u8 = 1;
pub const MY_SEED_U32: u32 = 2;
pub const MY_SEED_U64: u64 = 3;

#[program]
pub mod pda_derivation {
    use super::*;

    pub fn init_base(ctx: Context<InitBase>, data: u64, data_key: Pubkey) -> Result<()> {
        let base = &mut ctx.accounts.base;
        base.base_data = data;
        base.base_data_key = data_key;
        Ok(())
    }

    pub fn init_my_account(ctx: Context<InitMyAccount>, _seed_a: u8) -> Result<()> {
        ctx.accounts.account.data = 1337;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitBase<'info> {
    #[account(
        init,
        payer = payer,
        space = 8+8+32,
    )]
    base: Account<'info, BaseAccount>,
    #[account(mut)]
    payer: Signer<'info>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(seed_a: u8)]
pub struct InitMyAccount<'info> {
    base: Account<'info, BaseAccount>,
    base2: AccountInfo<'info>,
    #[account(
        init,
        payer = payer,
        space = 8+8,
        seeds = [
            &seed_a.to_le_bytes(),
            "another-seed".as_bytes(),
            b"test".as_ref(),
            base.key().as_ref(),
            base2.key.as_ref(),
            MY_SEED.as_ref(),
            MY_SEED_STR.as_bytes(),
            MY_SEED_U8.to_le_bytes().as_ref(),
            &MY_SEED_U32.to_le_bytes(),
            &MY_SEED_U64.to_le_bytes(),
            base.base_data.to_le_bytes().as_ref(),
            base.base_data_key.as_ref(),
        ],
        bump,
    )]
    account: Account<'info, MyAccount>,
    #[account(mut)]
    payer: Signer<'info>,
    system_program: Program<'info, System>,
}

#[account]
pub struct MyAccount {
    data: u64,
}

#[account]
pub struct BaseAccount {
    base_data: u64,
    base_data_key: Pubkey,
}
