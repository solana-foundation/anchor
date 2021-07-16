use crate::account::*;
use crate::misc::MyState;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount};
use misc2::misc2::MyState as Misc2State;

#[derive(Accounts)]
#[instruction(nonce: u8)]
pub struct TestTokenSeedsInit<'info> {
    #[account(
        init,
        token = mint,
        authority = authority,
        seeds = [b"my-token-seed".as_ref(), &[nonce]],
        payer = authority,
        space = TokenAccount::LEN,
    )]
    pub my_pda: CpiAccount<'info, TokenAccount>,
    pub mint: CpiAccount<'info, Mint>,
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
    pub rent: Sysvar<'info, Rent>,
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct TestPdaInitZeroCopy<'info> {
    #[account(init, seeds = [b"my-seed".as_ref(), &[bump]], payer = my_payer)]
    pub my_pda: Loader<'info, DataZeroCopy>,
    pub my_payer: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
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
    pub rent: Sysvar<'info, Rent>,
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

// `my_account` is the associated token account being created.
// `authority` must be a `mut` and `signer` since it will pay for the creation
// of the associated token account. `state` is used as an association, i.e., one
// can *optionally* identify targets to be used as seeds for the program
// derived address by using `with` (and it doesn't have to be a state account).
// For example, the SPL token program uses a `Mint` account. Lastly,
// `rent` and `system_program` are *required* by convention, since the
// accounts are needed when creating the associated program address within
// the program.
#[derive(Accounts)]
pub struct TestInitAssociatedAccount<'info> {
    #[account(init, associated = authority, with = state, with = data, with = b"my-seed")]
    pub my_account: ProgramAccount<'info, TestData>,
    #[account(mut, signer)]
    pub authority: AccountInfo<'info>,
    pub state: ProgramState<'info, MyState>,
    pub data: ProgramAccount<'info, Data>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestAssociatedAccount<'info> {
    #[account(mut, associated = authority, with = state, with = data, with = b"my-seed")]
    pub my_account: ProgramAccount<'info, TestData>,
    #[account(mut, signer)]
    pub authority: AccountInfo<'info>,
    pub state: ProgramState<'info, MyState>,
    pub data: ProgramAccount<'info, Data>,
}

#[derive(Accounts)]
pub struct TestU16<'info> {
    #[account(init)]
    pub my_account: ProgramAccount<'info, DataU16>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct TestI16<'info> {
    #[account(init)]
    pub data: ProgramAccount<'info, DataI16>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct TestSimulate {}

#[derive(Accounts)]
pub struct TestSimulateAssociatedAccount<'info> {
    #[account(init, associated = authority)]
    pub my_account: ProgramAccount<'info, TestData>,
    #[account(mut, signer)]
    pub authority: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TestI8<'info> {
    #[account(init)]
    pub data: ProgramAccount<'info, DataI8>,
    pub rent: Sysvar<'info, Rent>,
}
