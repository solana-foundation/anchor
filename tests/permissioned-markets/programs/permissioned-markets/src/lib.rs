// Note. This example depends on unreleased Serum DEX changes.

use anchor_lang::prelude::*;
use anchor_spl::dex;
use serum_dex::instruction::MarketInstruction;
use serum_dex::matching::Side;
use serum_dex::state::OpenOrders;
use solana_program::instruction::Instruction;
use solana_program::program;
use solana_program::system_program;
use solana_program::sysvar::rent;
use std::mem::size_of;

/// A low level example of permissioned markets.
///
/// It's recommended to instead study `programs/permissioned-markets-middleware`
/// in this workspace, which achieves the same functionality in a simpler, more
/// extendable fashion via a middleware abstraction. This program achieves
/// mostly the same proxy + middleware functionality, but in a much uglier way.
///
/// This example is provided as a (very) rough guide for how to might implement
/// a permissioned market in a raw program, which may be useful in the
/// unexpected case that the middleware abstraction does not fit one's use case.
///
/// Note that a fallback function is used here as the entrypoint instead of
/// higher level Anchor instruction handers. This is done to keep the example
/// consistent with `programs/permissioned-markets-middleware`. A program
/// with explicit instruction handlers would work, though then one would lose
/// the middleware abstraction, which may or may not be acceptable depending on
/// your use case.
#[program]
pub mod permissioned_markets {
    use super::*;

    #[access_control(is_serum(accounts))]
    pub fn dex_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        mut data: &[u8],
    ) -> ProgramResult {
        require!(!accounts.is_empty(), NotEnoughAccounts);

        // Strip instruction data.
        let bumps = {
            // Strip the discriminator off the data, which is provided by the client
            // for prepending extra instruction data.
            let disc = data[0];
            data = &data[1..];

            // For the init open orders instruction, bump seeds are provided.
            if disc == 0 {
                let bump = data[0];
                let bump_init = data[1];
                data = &data[2..]; // Strip bumps off.
                Some((bump, bump_init))
            } else {
                None
            }
        };

        // Strip accounts.
        let (dex, mut acc_infos) = {
            // First account is the dex executable--used for CPI.
            let dex = &accounts[0];

            // Second account is the auth token.
            let auth_token = &accounts[1];
            if auth_token.key != &rent::ID {
                // Rent sysvar as dummy example.
                return Err(ErrorCode::InvalidAuthToken.into());
            }

            // Strip.
            let acc_infos = (&accounts[2..]).to_vec();

            (dex, acc_infos)
        };

        let mut pre_instruction: Option<CpiInstruction> = None;
        let mut post_instruction: Option<CpiInstruction> = None;

        // Decode instruction.
        let ix = MarketInstruction::unpack(data).ok_or(ErrorCode::CannotUnpack)?;

        // Swap the user's account, which is in the open orders authority
        // position, for the program's PDA (the real authority).
        let (market, user) = match ix {
            MarketInstruction::InitOpenOrders => {
                let (market, user) = {
                    let market = &acc_infos[4];
                    let user = &acc_infos[3];

                    let (bump, bump_init) = bumps.as_ref().unwrap();

                    // Initialize PDA.
                    let mut accounts = &acc_infos[..];
                    InitAccount::try_accounts(program_id, &mut accounts, &[*bump, *bump_init])?;

                    (*market.key, *user.key)
                };
                // Chop off the first two accounts used initializing the PDA.
                acc_infos = (&acc_infos[2..]).to_vec();

                // Set signers.
                acc_infos[1] = prepare_pda(&acc_infos[0]);
                acc_infos[4].is_signer = true;

                (market, user)
            }
            MarketInstruction::NewOrderV3(ix) => {
                require!(acc_infos.len() >= 12, NotEnoughAccounts);

                let (market, user) = {
                    let market = &acc_infos[0];
                    let user = &acc_infos[7];

                    if !user.is_signer {
                        return Err(ErrorCode::UnauthorizedUser.into());
                    }

                    (*market.key, *user.key)
                };

                // Pre-instruction to approve delegate.
                {
                    let market = &acc_infos[0];
                    let user = &acc_infos[7];
                    let open_orders = &acc_infos[1];
                    let token_account_payer = &acc_infos[6];
                    let amount = match ix.side {
                        Side::Bid => ix.max_native_pc_qty_including_fees.get(),
                        Side::Ask => {
                            // +5 for padding.
                            let coin_lot_idx = 5 + 43 * 8;
                            let data = market.try_borrow_data()?;
                            let mut coin_lot_array = [0u8; 8];
                            coin_lot_array.copy_from_slice(&data[coin_lot_idx..coin_lot_idx + 8]);
                            let coin_lot_size = u64::from_le_bytes(coin_lot_array);
                            ix.max_coin_qty.get().checked_mul(coin_lot_size).unwrap()
                        }
                    };
                    let ix = spl_token::instruction::approve(
                        &spl_token::ID,
                        token_account_payer.key,
                        open_orders.key,
                        user.key,
                        &[],
                        amount,
                    )?;
                    let accounts = vec![
                        token_account_payer.clone(),
                        open_orders.clone(),
                        user.clone(),
                    ];
                    pre_instruction = Some((ix, accounts, Vec::new()));
                };

                // Post-instruction to revoke delegate.
                {
                    let user = &acc_infos[7];
                    let token_account_payer = &acc_infos[6];
                    let ix = spl_token::instruction::revoke(
                        &spl_token::ID,
                        token_account_payer.key,
                        user.key,
                        &[],
                    )?;
                    let accounts = vec![token_account_payer.clone(), user.clone()];
                    post_instruction = Some((ix, accounts, Vec::new()));
                }

                acc_infos[7] = prepare_pda(&acc_infos[1]);

                (market, user)
            }
            MarketInstruction::CancelOrderV2(_) => {
                require!(acc_infos.len() >= 6, NotEnoughAccounts);

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
                require!(acc_infos.len() >= 6, NotEnoughAccounts);

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
                require!(acc_infos.len() >= 10, NotEnoughAccounts);

                let (market, user) = {
                    let market = &acc_infos[0];
                    let user = &acc_infos[2];
                    let referral = &acc_infos[9];

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
                require!(acc_infos.len() >= 4, NotEnoughAccounts);

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

        // Execute pre instruction.
        if let Some((ix, accounts, seeds)) = pre_instruction {
            let tmp_signers: Vec<Vec<&[u8]>> = seeds
                .iter()
                .map(|seeds| {
                    let seeds: Vec<&[u8]> = seeds.iter().map(|seed| &seed[..]).collect();
                    seeds
                })
                .collect();
            let signers: Vec<&[&[u8]]> = tmp_signers.iter().map(|seeds| &seeds[..]).collect();
            program::invoke_signed(&ix, &accounts, &signers)?;
        }

        // CPI to the dex.
        let dex_accounts = acc_infos
            .iter()
            .map(|acc| AccountMeta {
                pubkey: *acc.key,
                is_signer: acc.is_signer,
                is_writable: acc.is_writable,
            })
            .collect();
        let ix = Instruction {
            data: data.to_vec(),
            accounts: dex_accounts,
            program_id: dex::ID,
        };
        let seeds = open_orders_authority! {
            program = program_id,
            dex_program = dex.key,
            market = market,
            authority = user
        };
        let seeds_init = open_orders_init_authority! {
            program = program_id,
            dex_program = dex.key,
            market = market
        };
        program::invoke_signed(&ix, &acc_infos, &[seeds, seeds_init])?;

        // Execute post instruction.
        if let Some((ix, accounts, seeds)) = post_instruction {
            let tmp_signers: Vec<Vec<&[u8]>> = seeds
                .iter()
                .map(|seeds| {
                    let seeds: Vec<&[u8]> = seeds.iter().map(|seed| &seed[..]).collect();
                    seeds
                })
                .collect();
            let signers: Vec<&[&[u8]]> = tmp_signers.iter().map(|seeds| &seeds[..]).collect();
            program::invoke_signed(&ix, &accounts, &signers)?;
        }

        Ok(())
    }
}

// Accounts context.

#[derive(Accounts)]
#[instruction(bump: u8, bump_init: u8)]
pub struct InitAccount<'info> {
    #[account(address = dex::ID)]
    pub dex_program: AccountInfo<'info>,
    #[account(address = system_program::ID)]
    pub system_program: AccountInfo<'info>,
    #[account(
        init,
        seeds = [b"open-orders", dex_program.key.as_ref(), market.key.as_ref(), authority.key.as_ref()],
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
    #[account(
        seeds = [b"open-orders-init", dex_program.key.as_ref(), market.key.as_ref()],
        bump = bump_init,
    )]
    pub open_orders_init_authority: AccountInfo<'info>,
}

// Access control modifiers.

fn is_serum(accounts: &[AccountInfo]) -> Result<()> {
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
    #[msg("Invalid auth token provided")]
    InvalidAuthToken,
}

// Utils.

fn prepare_pda<'info>(acc_info: &AccountInfo<'info>) -> AccountInfo<'info> {
    let mut acc_info = acc_info.clone();
    acc_info.is_signer = true;
    acc_info
}

// Macros.

/// Returns the seeds used for a user's open orders account PDA.
#[macro_export]
macro_rules! open_orders_authority {
    (
        program = $program:expr,
        dex_program = $dex_program:expr,
        market = $market:expr,
        authority = $authority:expr,
        bump = $bump:expr
    ) => {
        &[
            b"open-orders".as_ref(),
            $dex_program.as_ref(),
            $market.as_ref(),
            $authority.as_ref(),
            &[$bump],
        ]
    };
    (
        program = $program:expr,
        dex_program = $dex_program:expr,
        market = $market:expr,
        authority = $authority:expr
    ) => {
        &[
            b"open-orders".as_ref(),
            $dex_program.as_ref(),
            $market.as_ref(),
            $authority.as_ref(),
            &[Pubkey::find_program_address(
                &[
                    b"open-orders".as_ref(),
                    $dex_program.as_ref(),
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
    (
        program = $program:expr,
        dex_program = $dex_program:expr,
        market = $market:expr,
        bump = $bump:expr
    ) => {
        &[
            b"open-orders-init".as_ref(),
            $dex_program.as_ref().as_ref(),
            $market.as_ref().as_ref(),
            &[$bump],
        ]
    };

    (program = $program:expr, dex_program = $dex_program:expr, market = $market:expr) => {
        &[
            b"open-orders-init".as_ref(),
            $dex_program.as_ref(),
            $market.as_ref(),
            &[Pubkey::find_program_address(
                &[
                    b"open-orders-init".as_ref(),
                    $dex_program.as_ref(),
                    $market.as_ref(),
                ],
                $program,
            )
            .1],
        ]
    };
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

type CpiInstruction<'info> = (Instruction, Vec<AccountInfo<'info>>, Vec<Vec<Vec<u8>>>);
