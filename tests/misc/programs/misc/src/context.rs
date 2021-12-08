use crate::account::*;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};
use misc2::misc2::MyState as Misc2State;
use std::mem::size_of;

#[derive(Accounts)]
#[instruction(token_bump: u8, mint_bump: u8)]
pub struct TestTokenSeedsInit<'info> {
    #[account(
        init,
        seeds = [b"my-mint-seed".as_ref()],
        bump = mint_bump,
        payer = authority,
        mint::decimals = 6,
        mint::authority = authority,
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        seeds = [b"my-token-seed".as_ref(),],
        bump = token_bump,
        payer = authority,
        token::mint = mint,
        token::authority = authority,
    )]
    pub my_pda: Account<'info, TokenAccount>,
    pub authority: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestInitAssociatedToken<'info> {
    #[account(
        init,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = payer,
    )]
    pub token: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    pub payer: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct TestValidateAssociatedToken<'info> {
    #[account(
        associated_token::mint = mint,
        associated_token::authority = wallet,
    )]
    pub token: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    pub wallet: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(nonce: u8)]
pub struct TestInstructionConstraint<'info> {
    #[account(
        seeds = [b"my-seed", my_account.key.as_ref()],
        bump = nonce,
    )]
    pub my_pda: AccountInfo<'info>,
    pub my_account: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(domain: String, seed: Vec<u8>, bump: u8)]
pub struct TestPdaInit<'info> {
    #[account(
        init,
        seeds = [b"my-seed", domain.as_bytes(), foo.key.as_ref(), &seed],
        bump = bump,
        payer = my_payer,
    )]
    pub my_pda: Account<'info, DataU16>,
    pub my_payer: AccountInfo<'info>,
    pub foo: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct TestPdaInitZeroCopy<'info> {
    #[account(
        init,
        seeds = [b"my-seed".as_ref()],
        bump = bump,
        payer = my_payer,
    )]
    pub my_pda: Loader<'info, DataZeroCopy>,
    pub my_payer: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestPdaMutZeroCopy<'info> {
    #[account(
        mut,
        seeds = [b"my-seed".as_ref()],
        bump = my_pda.load()?.bump,
    )]
    pub my_pda: Loader<'info, DataZeroCopy>,
    pub my_payer: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Ctor {}

#[derive(Accounts)]
pub struct RemainingAccounts {}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(zero)]
    pub data: Account<'info, Data>,
}

#[derive(Accounts)]
pub struct TestOwner<'info> {
    #[account(owner = *misc.key)]
    pub data: AccountInfo<'info>,
    pub misc: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestExecutable<'info> {
    #[account(executable)]
    pub program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestStateCpi<'info> {
    #[account(signer)]
    pub authority: AccountInfo<'info>,
    #[account(mut, state = misc2_program)]
    pub cpi_state: CpiState<'info, Misc2State>,
    #[account(executable)]
    pub misc2_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestClose<'info> {
    #[account(mut, close = sol_dest)]
    pub data: Account<'info, Data>,
    sol_dest: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestU16<'info> {
    #[account(zero)]
    pub my_account: Account<'info, DataU16>,
}

#[derive(Accounts)]
pub struct TestI16<'info> {
    #[account(zero)]
    pub data: Account<'info, DataI16>,
}

#[derive(Accounts)]
pub struct TestSimulate {}

#[derive(Accounts)]
pub struct TestI8<'info> {
    #[account(zero)]
    pub data: Account<'info, DataI8>,
}

#[derive(Accounts)]
pub struct TestInit<'info> {
    #[account(init, payer = payer)]
    pub data: Account<'info, DataI8>,
    #[account(signer)]
    pub payer: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestInitZeroCopy<'info> {
    #[account(init, payer = payer, space = 8 + size_of::<DataZeroCopy>())]
    pub data: Loader<'info, DataZeroCopy>,
    #[account(signer)]
    pub payer: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestInitMint<'info> {
    #[account(init, mint::decimals = 6, mint::authority = payer, mint::freeze_authority = payer, payer = payer)]
    pub mint: Account<'info, Mint>,
    #[account(signer)]
    pub payer: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestInitToken<'info> {
    #[account(init, token::mint = mint, token::authority = payer, payer = payer)]
    pub token: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    #[account(signer)]
    pub payer: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestCompositePayer<'info> {
    pub composite: TestInit<'info>,
    #[account(init, payer = composite.payer, space = 8 + size_of::<Data>())]
    pub data: Account<'info, Data>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TestFetchAll<'info> {
    #[account(init, payer = authority)]
    pub data: Account<'info, DataWithFilter>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TestInitWithEmptySeeds<'info> {
    #[account(init, seeds = [], bump, payer = authority, space = 8 + size_of::<Data>())]
    pub pda: Account<'info, Data>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TestEmptySeedsConstraint<'info> {
    #[account(seeds = [], bump)]
    pub pda: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct InitWithSpace<'info> {
    #[account(init, payer = payer)]
    pub data: Account<'info, DataU16>,
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TestInitIfNeeded<'info> {
    #[account(init_if_needed, payer = payer, space = 500)]
    pub data: Account<'info, DataU16>,
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TestInitIfNeededChecksOwner<'info> {
    #[account(init_if_needed, payer = payer, space = 100, owner = *owner.key, seeds = [b"hello"], bump)]
    pub data: UncheckedAccount<'info>,
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub owner: AccountInfo<'info>
}

#[derive(Accounts)]
#[instruction(seed_data: String)]
pub struct TestInitIfNeededChecksSeeds<'info> {
    #[account(init_if_needed, payer = payer, space = 100, seeds = [seed_data.as_bytes()], bump)]
    pub data: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(decimals: u8)]
pub struct TestInitMintIfNeeded<'info> {
    #[account(init_if_needed, mint::decimals = decimals, mint::authority = mint_authority, mint::freeze_authority = freeze_authority, payer = payer)]
    pub mint: Account<'info, Mint>,
    #[account(signer)]
    pub payer: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub mint_authority: AccountInfo<'info>,
    pub freeze_authority: AccountInfo<'info>, 
}

#[derive(Accounts)]
pub struct TestInitTokenIfNeeded<'info> {
    #[account(init_if_needed, token::mint = mint, token::authority = authority, payer = payer)]
    pub token: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    #[account(signer)]
    pub payer: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestInitAssociatedTokenIfNeeded<'info> {
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = authority,
    )]
    pub token: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    pub payer: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub authority: AccountInfo<'info>
}

#[derive(Accounts)]
pub struct TestMultidimensionalArray<'info> {
    #[account(zero)]
    pub data: Account<'info, DataMultidimensionalArray>,
}
