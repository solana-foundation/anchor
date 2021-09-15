use crate::{dex, open_orders_authority, open_orders_init_authority, token};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::system_program;
use anchor_lang::Accounts;
use serum_dex::instruction::*;
use serum_dex::matching::Side;
use serum_dex::state::OpenOrders;
use std::mem::size_of;

/// Per request context. Can be used to share data between middleware handlers.
pub struct Context<'a, 'info> {
    pub program_id: &'a Pubkey,
    pub dex_program_id: &'a Pubkey,
    pub accounts: Vec<AccountInfo<'info>>,
    pub seeds: Seeds,
    // Instructions to execute *prior* to the DEX relay CPI.
    pub pre_instructions: Vec<(Instruction, Vec<AccountInfo<'info>>, Seeds)>,
    // Instructions to execution *after* the DEX relay CPI.
    pub post_instructions: Vec<(Instruction, Vec<AccountInfo<'info>>, Seeds)>,
}

type Seeds = Vec<Vec<Vec<u8>>>;

impl<'a, 'info> Context<'a, 'info> {
    pub fn new(
        program_id: &'a Pubkey,
        dex_program_id: &'a Pubkey,
        accounts: Vec<AccountInfo<'info>>,
    ) -> Self {
        Self {
            program_id,
            dex_program_id,
            accounts,
            seeds: Vec::new(),
            pre_instructions: Vec::new(),
            post_instructions: Vec::new(),
        }
    }
}

/// Implementing this trait allows one to hook into requests to the Serum DEX
/// via a frontend proxy.
pub trait MarketMiddleware {
    /// Called before any instruction, giving middleware access to the raw
    /// instruction data. This can be used to access extra data that is
    /// prepended to the DEX data, allowing one to expand the capabilities of
    /// any instruction by reading the instruction data here and then
    /// using it in any of the method handlers.
    fn instruction(&mut self, _data: &mut &[u8]) -> ProgramResult {
        Ok(())
    }

    fn init_open_orders(&self, _ctx: &mut Context) -> ProgramResult {
        Ok(())
    }

    fn new_order_v3(&self, _ctx: &mut Context, _ix: &NewOrderInstructionV3) -> ProgramResult {
        Ok(())
    }

    fn cancel_order_v2(&self, _ctx: &mut Context, _ix: &CancelOrderInstructionV2) -> ProgramResult {
        Ok(())
    }

    fn cancel_order_by_client_id_v2(&self, _ctx: &mut Context, _client_id: u64) -> ProgramResult {
        Ok(())
    }

    fn settle_funds(&self, _ctx: &mut Context) -> ProgramResult {
        Ok(())
    }

    fn close_open_orders(&self, _ctx: &mut Context) -> ProgramResult {
        Ok(())
    }

    fn prune(&self, _ctx: &mut Context, _limit: u16) -> ProgramResult {
        Ok(())
    }

    /// Called when the instruction data doesn't match any DEX instruction.
    fn fallback(&self, _ctx: &mut Context) -> ProgramResult {
        Ok(())
    }
}

/// Checks that the given open orders account signs the transaction and then
/// replaces it with the open orders account, which must be a PDA.
#[derive(Default)]
pub struct OpenOrdersPda {
    bump: u8,
    bump_init: u8,
}

impl OpenOrdersPda {
    pub fn new() -> Self {
        Self {
            bump: 0,
            bump_init: 0,
        }
    }
    fn prepare_pda<'info>(acc_info: &AccountInfo<'info>) -> AccountInfo<'info> {
        let mut acc_info = acc_info.clone();
        acc_info.is_signer = true;
        acc_info
    }
}

impl MarketMiddleware for OpenOrdersPda {
    fn instruction(&mut self, data: &mut &[u8]) -> ProgramResult {
        // Strip the discriminator.
        let disc = data[0];
        *data = &data[1..];

        // Discriminator == 0 implies it's the init instruction.
        if disc == 0 {
            self.bump = data[0];
            self.bump_init = data[1];
            *data = &data[2..];
        }
        Ok(())
    }

    /// Accounts:
    ///
    /// 0. Dex program.
    /// 1. System program.
    /// .. serum_dex::MarketInstruction::InitOpenOrders.
    ///
    /// Data:
    ///
    /// 0.   Discriminant.
    /// 1..2 Borsh(struct { bump: u8, bump_init: u8 }).
    /// ..
    fn init_open_orders<'a, 'info>(&self, ctx: &mut Context<'a, 'info>) -> ProgramResult {
        let market = &ctx.accounts[4];
        let user = &ctx.accounts[3];

        // Initialize PDA.
        let mut accounts = &ctx.accounts[..];
        InitAccount::try_accounts(ctx.program_id, &mut accounts, &[self.bump, self.bump_init])?;

        // Add signer to context.
        ctx.seeds.push(open_orders_authority! {
            program = ctx.program_id,
            dex_program = ctx.dex_program_id,
            market = market.key,
            authority = user.key,
            bump = self.bump
        });
        ctx.seeds.push(open_orders_init_authority! {
            program = ctx.program_id,
            dex_program = ctx.dex_program_id,
            market = market.key,
            bump = self.bump_init
        });

        // Chop off the first two accounts needed for initializing the PDA.
        ctx.accounts = (&ctx.accounts[2..]).to_vec();

        // Set PDAs.
        ctx.accounts[1] = Self::prepare_pda(&ctx.accounts[0]);
        ctx.accounts[4].is_signer = true;

        Ok(())
    }

    /// Accounts:
    ///
    /// ..
    ///
    /// Data:
    ///
    /// 0.   Discriminant.
    /// ..
    fn new_order_v3(&self, ctx: &mut Context, ix: &NewOrderInstructionV3) -> ProgramResult {
        // The user must authorize the tx.
        let user = &ctx.accounts[7];
        if !user.is_signer {
            return Err(ErrorCode::UnauthorizedUser.into());
        }

        let market = &ctx.accounts[0];
        let open_orders = &ctx.accounts[1];
        let token_account_payer = &ctx.accounts[6];

        // Pre: Give the PDA delegate access.
        let pre_instruction = {
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
            (ix, accounts, Vec::new())
        };
        ctx.pre_instructions.push(pre_instruction);

        // Post: Revoke the PDA's delegate access.
        let post_instruction = {
            let ix = spl_token::instruction::revoke(
                &spl_token::ID,
                token_account_payer.key,
                user.key,
                &[],
            )?;
            let accounts = vec![token_account_payer.clone(), user.clone()];
            (ix, accounts, Vec::new())
        };
        ctx.post_instructions.push(post_instruction);

        // Proxy: PDA must sign the new order.
        ctx.seeds.push(open_orders_authority! {
            program = ctx.program_id,
            dex_program = ctx.dex_program_id,
            market = market.key,
            authority = user.key
        });
        ctx.accounts[7] = Self::prepare_pda(open_orders);

        Ok(())
    }

    /// Accounts:
    ///
    /// ..
    ///
    /// Data:
    ///
    /// 0.   Discriminant.
    /// ..
    fn cancel_order_v2(&self, ctx: &mut Context, _ix: &CancelOrderInstructionV2) -> ProgramResult {
        let market = &ctx.accounts[0];
        let user = &ctx.accounts[4];
        if !user.is_signer {
            return Err(ErrorCode::UnauthorizedUser.into());
        }

        ctx.seeds.push(open_orders_authority! {
            program = ctx.program_id,
            dex_program = ctx.dex_program_id,
            market = market.key,
            authority = user.key
        });

        ctx.accounts[4] = Self::prepare_pda(&ctx.accounts[3]);

        Ok(())
    }

    /// Accounts:
    ///
    /// ..
    ///
    /// Data:
    ///
    /// 0.   Discriminant.
    /// ..
    fn cancel_order_by_client_id_v2(&self, ctx: &mut Context, _client_id: u64) -> ProgramResult {
        let market = &ctx.accounts[0];
        let user = &ctx.accounts[4];
        if !user.is_signer {
            return Err(ErrorCode::UnauthorizedUser.into());
        }

        ctx.seeds.push(open_orders_authority! {
            program = ctx.program_id,
            dex_program = ctx.dex_program_id,
            market = market.key,
            authority = user.key
        });

        ctx.accounts[4] = Self::prepare_pda(&ctx.accounts[3]);

        Ok(())
    }

    /// Accounts:
    ///
    /// ..
    ///
    /// Data:
    ///
    /// 0.   Discriminant.
    /// ..
    fn settle_funds(&self, ctx: &mut Context) -> ProgramResult {
        let market = &ctx.accounts[0];
        let user = &ctx.accounts[2];
        if !user.is_signer {
            return Err(ErrorCode::UnauthorizedUser.into());
        }

        ctx.seeds.push(open_orders_authority! {
            program = ctx.program_id,
            dex_program = ctx.dex_program_id,
            market = market.key,
            authority = user.key
        });

        ctx.accounts[2] = Self::prepare_pda(&ctx.accounts[1]);

        Ok(())
    }

    /// Accounts:
    ///
    /// ..
    ///
    /// Data:
    ///
    /// 0.   Discriminant.
    /// ..
    fn close_open_orders(&self, ctx: &mut Context) -> ProgramResult {
        let market = &ctx.accounts[3];
        let user = &ctx.accounts[1];
        if !user.is_signer {
            return Err(ErrorCode::UnauthorizedUser.into());
        }

        ctx.seeds.push(open_orders_authority! {
            program = ctx.program_id,
            dex_program = ctx.dex_program_id,
            market = market.key,
            authority = user.key
        });

        ctx.accounts[1] = Self::prepare_pda(&ctx.accounts[0]);

        Ok(())
    }

    /// Accounts:
    ///
    /// ..
    ///
    /// Data:
    ///
    /// 0.   Discriminant.
    /// ..
    fn prune(&self, ctx: &mut Context, _limit: u16) -> ProgramResult {
        // Set owner of open orders to be itself.
        ctx.accounts[5] = ctx.accounts[4].clone();
        Ok(())
    }
}

/// Logs each request.
pub struct Logger;
impl MarketMiddleware for Logger {
    fn init_open_orders(&self, _ctx: &mut Context) -> ProgramResult {
        msg!("proxying open orders");
        Ok(())
    }

    fn new_order_v3(&self, _ctx: &mut Context, ix: &NewOrderInstructionV3) -> ProgramResult {
        msg!("proxying new order v3 {:?}", ix);
        Ok(())
    }

    fn cancel_order_v2(&self, _ctx: &mut Context, ix: &CancelOrderInstructionV2) -> ProgramResult {
        msg!("proxying cancel order v2 {:?}", ix);
        Ok(())
    }

    fn cancel_order_by_client_id_v2(&self, _ctx: &mut Context, client_id: u64) -> ProgramResult {
        msg!("proxying cancel order by client id v2 {:?}", client_id);
        Ok(())
    }

    fn settle_funds(&self, _ctx: &mut Context) -> ProgramResult {
        msg!("proxying settle funds");
        Ok(())
    }

    fn close_open_orders(&self, _ctx: &mut Context) -> ProgramResult {
        msg!("proxying close open orders");
        Ok(())
    }

    fn prune(&self, _ctx: &mut Context, limit: u16) -> ProgramResult {
        msg!("proxying prune {:?}", limit);
        Ok(())
    }
}

/// Enforces referal fees being sent to the configured address.
pub struct ReferralFees {
    referral: Pubkey,
}

impl ReferralFees {
    pub fn new(referral: Pubkey) -> Self {
        Self { referral }
    }
}

impl MarketMiddleware for ReferralFees {
    /// Accounts:
    ///
    /// .. serum_dex::MarketInstruction::SettleFunds.
    fn settle_funds(&self, ctx: &mut Context) -> ProgramResult {
        let referral = token::accessor::authority(&ctx.accounts[9])?;
        require!(referral == self.referral, ErrorCode::InvalidReferral);
        Ok(())
    }
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
        vec![
            b"open-orders".to_vec(),
            $dex_program.as_ref().to_vec(),
            $market.as_ref().to_vec(),
            $authority.as_ref().to_vec(),
            vec![$bump],
        ]
    };
    (
        program = $program:expr,
        dex_program = $dex_program:expr,
        market = $market:expr,
        authority = $authority:expr
    ) => {
        vec![
            b"open-orders".to_vec(),
            $dex_program.as_ref().to_vec(),
            $market.as_ref().to_vec(),
            $authority.as_ref().to_vec(),
            vec![
                Pubkey::find_program_address(
                    &[
                        b"open-orders".as_ref(),
                        $dex_program.as_ref(),
                        $market.as_ref(),
                        $authority.as_ref(),
                    ],
                    $program,
                )
                .1,
            ],
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
        vec![
            b"open-orders-init".to_vec(),
            $dex_program.as_ref().to_vec(),
            $market.as_ref().to_vec(),
            vec![$bump],
        ]
    };
}

// Errors.

#[error(offset = 500)]
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
    #[msg("Invalid target program ID")]
    InvalidTargetProgram,
}

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

// Constants.

// Padding added to every serum account.
//
// b"serum".len() + b"padding".len().
const SERUM_PADDING: usize = 12;
