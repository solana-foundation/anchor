use crate::account::*;
use crate::misc::MyState;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount};
use misc2::misc2::MyState as Misc2State;

#[derive(Accounts)]
#[instruction(token_bump: u8, mint_bump: u8)]
pub struct TestTokenSeedsInit<'info> {
    #[account(
        init,
        seeds = [b"my-mint-seed".as_ref(), &[mint_bump]],
        payer = authority,
        mint::decimals = 6,
        mint::authority = authority,
    )]
    pub mint: CpiAccount<'info, Mint>,
    #[account(
        init,
        seeds = [b"my-token-seed".as_ref(), &[token_bump]],
        payer = authority,
        token::mint = mint,
        token::authority = authority,
    )]
    pub my_pda: CpiAccount<'info, TokenAccount>,
    pub authority: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(nonce: u8)]
pub struct TestInstructionConstraint<'info> {
    #[account(seeds = [b"my-seed", my_account.key.as_ref(), &[nonce]])]
    pub my_pda: AccountInfo<'info>,
    pub my_account: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(domain: String, seed: Vec<u8>, bump: u8)]
pub struct TestPdaInit<'info> {
    #[account(
        init,
        seeds = [b"my-seed", domain.as_bytes(), foo.key.as_ref(), &seed, &[bump]],
        payer = my_payer,
    )]
    pub my_pda: ProgramAccount<'info, DataU16>,
    pub my_payer: AccountInfo<'info>,
    pub foo: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct TestPdaInitZeroCopy<'info> {
    #[account(init, seeds = [b"my-seed".as_ref(), &[bump]], payer = my_payer)]
    pub my_pda: Loader<'info, DataZeroCopy>,
    pub my_payer: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestPdaMutZeroCopy<'info> {
    #[account(mut, seeds = [b"my-seed".as_ref(), &[my_pda.load()?.bump]])]
    pub my_pda: Loader<'info, DataZeroCopy>,
    pub my_payer: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Ctor {}

#[derive(Accounts)]
pub struct RemainingAccounts {}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init)]
    pub data: ProgramAccount<'info, Data>,
}

#[derive(Accounts)]
pub struct TestOwner<'info> {
    #[account(owner = misc)]
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
    pub data: ProgramAccount<'info, Data>,
    sol_dest: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestU16<'info> {
    #[account(init)]
    pub my_account: ProgramAccount<'info, DataU16>,
}

#[derive(Accounts)]
pub struct TestI16<'info> {
    #[account(init)]
    pub data: ProgramAccount<'info, DataI16>,
}

#[derive(Accounts)]
pub struct TestSimulate {}

#[derive(Accounts)]
pub struct TestI8<'info> {
    #[account(init)]
    pub data: ProgramAccount<'info, DataI8>,
}
