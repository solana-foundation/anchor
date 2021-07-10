// Note. This example depends on unreleased Serum DEX changes.

use anchor_lang::prelude::*;
use anchor_spl::dex;
use serum_dex::instruction::MarketInstruction;
use serum_dex::state::OpenOrders;
use solana_program::instruction::Instruction;
use solana_program::program;
use solana_program::system_program;
use std::mem::size_of;

/// This demonstrates how to create "permissioned markets" on Serum. A
/// permissioned market is a regular Serum market with an additional
/// open orders authority, which must sign every transaction to create or
/// close an open orders account.
///
/// In practice, what this means is that one can create a program that acts
/// as this authority *and* that marks its own PDAs as the *owner* of all
/// created open orders accounts, making the program the sole arbiter over
/// who can trade on a given market.
///
/// For example, this example forces all trades that execute on this market
/// to set the referral to a hardcoded address, i.e., `fee_owner::ID`.
#[program]
pub mod permissioned_markets {
    use super::*;

    /// Creates an open orders account controlled by this program on behalf of
    /// the user.
    ///
    /// Note that although the owner of the open orders account is the dex
    /// program, This instruction must be executed within this program, rather
    /// than a relay, because it initializes a PDA.
    pub fn init_account(ctx: Context<InitAccount>, bump: u8, bump_init: u8) -> Result<()> {
        let cpi_ctx = CpiContext::from(&*ctx.accounts);
        let seeds = open_orders_authority! {
            program = ctx.program_id,
            market = ctx.accounts.market.key,
            authority = ctx.accounts.authority.key,
            bump = bump
        };
        let seeds_init = open_orders_init_authority! {
            program = ctx.program_id,
            market = ctx.accounts.market.key,
            bump = bump_init
        };
        dex::init_open_orders(cpi_ctx.with_signer(&[seeds, seeds_init]))?;
        Ok(())
    }

    /// Fallback function to relay calls to the serum DEX.
    ///
    /// For instructions requiring an open orders authority, checks for
    /// a user signature and then swaps the account info for one controlled
    /// by the program.
    ///
    /// Note: the "authority" of each open orders account is the account
    ///       itself, since it's a PDA.
    #[access_control(is_serum(accounts))]
    pub fn dex_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        data: &[u8],
    ) -> ProgramResult {
        require!(accounts.len() >= 1, NotEnoughAccounts);

        let dex_acc_info = &accounts[0];
        let dex_accounts = &accounts[1..];
        let mut acc_infos = dex_accounts.to_vec();

        // Decode instruction.
        let ix = MarketInstruction::unpack(data).ok_or_else(|| ErrorCode::CannotUnpack)?;

        // Swap the user's account, which is in the open orders authority
        // position, for the program's PDA (the real authority).
        let (market, user) = match ix {
            MarketInstruction::NewOrderV3(_) => {
                require!(dex_accounts.len() >= 12, NotEnoughAccounts);

                let (market, user) = {
                    let market = &acc_infos[0];
                    let user = &acc_infos[7];

                    if !user.is_signer {
                        return Err(ErrorCode::UnauthorizedUser.into());
                    }

                    (*market.key, *user.key)
                };

                acc_infos[7] = prepare_pda(&acc_infos[1]);

                (market, user)
            }
            MarketInstruction::CancelOrderV2(_) => {
                require!(dex_accounts.len() >= 6, NotEnoughAccounts);

                let (market, user) = {
                    let market = &acc_infos[0];
                    let user = &acc_infos[4];

                    if !user.is_signer {
                        return Err(ErrorCode::UnauthorizedUser.into());
                    }

                    (*market.key, *user.key)
                };

                acc_infos[4] = prepare_pda(&acc_infos[3]);

                (market, user)
            }
            MarketInstruction::CancelOrderByClientIdV2(_) => {
                require!(dex_accounts.len() >= 6, NotEnoughAccounts);

                let (market, user) = {
                    let market = &acc_infos[0];
                    let user = &acc_infos[4];

                    if !user.is_signer {
                        return Err(ErrorCode::UnauthorizedUser.into());
                    }

                    (*market.key, *user.key)
                };

                acc_infos[4] = prepare_pda(&acc_infos[3]);

                (market, user)
            }
            MarketInstruction::SettleFunds => {
                require!(dex_accounts.len() >= 10, NotEnoughAccounts);

                let (market, user) = {
                    let market = &acc_infos[0];
                    let user = &acc_infos[2];
                    let referral = &dex_accounts[9];

                    if !DISABLE_REFERRAL && referral.key != &referral::ID {
                        return Err(ErrorCode::InvalidReferral.into());
                    }
                    if !user.is_signer {
                        return Err(ErrorCode::UnauthorizedUser.into());
                    }

                    (*market.key, *user.key)
                };

                acc_infos[2] = prepare_pda(&acc_infos[1]);

                (market, user)
            }
            MarketInstruction::CloseOpenOrders => {
                require!(dex_accounts.len() >= 4, NotEnoughAccounts);

                let (market, user) = {
                    let market = &acc_infos[3];
                    let user = &acc_infos[1];

                    if !user.is_signer {
                        return Err(ErrorCode::UnauthorizedUser.into());
                    }

                    (*market.key, *user.key)
                };

                acc_infos[1] = prepare_pda(&acc_infos[0]);

                (market, user)
            }
            _ => return Err(ErrorCode::InvalidInstruction.into()),
        };

        // CPI to the dex.
        let dex_accounts = acc_infos
            .iter()
            .map(|acc| AccountMeta {
                pubkey: *acc.key,
                is_signer: acc.is_signer,
                is_writable: acc.is_writable,
            })
            .collect();
        acc_infos.push(dex_acc_info.clone());
        let ix = Instruction {
            data: data.to_vec(),
            accounts: dex_accounts,
            program_id: dex::ID,
        };
        let seeds = open_orders_authority! {
            program = program_id,
            market = market,
            authority = user
        };
        program::invoke_signed(&ix, &acc_infos, &[seeds])
    }
}

// Accounts context.

#[derive(Accounts)]
#[instruction(bump: u8, bump_init: u8)]
pub struct InitAccount<'info> {
    #[account(seeds = [b"open-orders-init", market.key.as_ref(), &[bump_init]])]
    pub open_orders_init_authority: AccountInfo<'info>,
    #[account(
        init,
        seeds = [b"open-orders", market.key.as_ref(), authority.key.as_ref()],
        bump = bump,
        payer = authority,
        owner = dex::ID,
        space = size_of::<OpenOrders>() + SERUM_PADDING,
    )]
    pub open_orders: AccountInfo<'info>,
    #[account(signer)]
    pub authority: AccountInfo<'info>,
    pub market: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    #[account(address = system_program::ID)]
    pub system_program: AccountInfo<'info>,
    #[account(address = dex::ID)]
    pub dex_program: AccountInfo<'info>,
}

// CpiContext transformations.

impl<'info> From<&InitAccount<'info>>
    for CpiContext<'_, '_, '_, 'info, dex::InitOpenOrders<'info>>
{
    fn from(accs: &InitAccount<'info>) -> Self {
        // TODO: add the open orders init authority account here once the
        //       dex is upgraded.
        let accounts = dex::InitOpenOrders {
            open_orders: accs.open_orders.clone(),
            authority: accs.open_orders.clone(),
            market: accs.market.clone(),
            rent: accs.rent.to_account_info(),
        };
        let program = accs.dex_program.clone();
        CpiContext::new(program, accounts)
    }
}

// Access control modifiers.

fn is_serum<'info>(accounts: &[AccountInfo<'info>]) -> Result<()> {
    let dex_acc_info = &accounts[0];
    if dex_acc_info.key != &dex::ID {
        return Err(ErrorCode::InvalidDexPid.into());
    }
    Ok(())
}

// Error.

#[error]
pub enum ErrorCode {
    #[msg("Program ID does not match the Serum DEX")]
    InvalidDexPid,
    #[msg("Invalid instruction given")]
    InvalidInstruction,
    #[msg("Could not unpack the instruction")]
    CannotUnpack,
    #[msg("Invalid referral address given")]
    InvalidReferral,
    #[msg("The user didn't sign")]
    UnauthorizedUser,
    #[msg("Not enough accounts were provided")]
    NotEnoughAccounts,
}

// Macros.

/// Returns the seeds used for creating the open orders account PDA.
#[macro_export]
macro_rules! open_orders_authority {
    (program = $program:expr, market = $market:expr, authority = $authority:expr, bump = $bump:expr) => {
        &[
            b"open-orders".as_ref(),
            $market.as_ref(),
            $authority.as_ref(),
            &[$bump],
        ]
    };
    (program = $program:expr, market = $market:expr, authority = $authority:expr) => {
        &[
            b"open-orders".as_ref(),
            $market.as_ref(),
            $authority.as_ref(),
            &[Pubkey::find_program_address(
                &[
                    b"open-orders".as_ref(),
                    $market.as_ref(),
                    $authority.as_ref(),
                ],
                $program,
            )
            .1],
        ]
    };
}

/// Returns the seeds used for the open orders init authority.
/// This is the account that must sign to create a new open orders account on
/// the DEX market.
#[macro_export]
macro_rules! open_orders_init_authority {
    (program = $program:expr, market = $market:expr) => {
        &[
            b"open-orders-init".as_ref(),
            $market.as_ref(),
            &[Pubkey::find_program_address(
                &[b"open-orders-init".as_ref(), $market.as_ref()],
                $program,
            )
            .1],
        ]
    };
    (program = $program:expr, market = $market:expr, bump = $bump:expr) => {
        &[b"open-orders-init".as_ref(), $market.as_ref(), &[$bump]]
    };
}

// Utils.

fn prepare_pda<'info>(acc_info: &AccountInfo<'info>) -> AccountInfo<'info> {
    let mut acc_info = acc_info.clone();
    acc_info.is_signer = true;
    acc_info
}

// Constants.

// Padding added to every serum account.
//
// b"serum".len() + b"padding".len().
const SERUM_PADDING: usize = 12;

// True if we don't care about referral access control (for testing).
const DISABLE_REFERRAL: bool = true;

/// The address that will receive all fees for all markets controlled by this
/// program. Note: this is a dummy address. Do not use in production.
pub mod referral {
    solana_program::declare_id!("2k1bb16Hu7ocviT2KC3wcCgETtnC8tEUuvFBH4C5xStG");
}
