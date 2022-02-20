use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_lang::solana_program::program_error::ProgramError;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts, Result, ToAccountInfos};
use serum_dex::instruction::SelfTradeBehavior;
use serum_dex::matching::{OrderType, Side};
use std::num::NonZeroU64;

pub use serum_dex;

#[cfg(not(feature = "devnet"))]
anchor_lang::solana_program::declare_id!("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin");

#[cfg(feature = "devnet")]
anchor_lang::solana_program::declare_id!("DESVgJVGajEgKGXhb6XmqDHGz3VjdgP7rEVESBgxmroY");

#[allow(clippy::too_many_arguments)]
pub fn new_order_v3<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, NewOrderV3<'info>>,
    side: Side,
    limit_price: NonZeroU64,
    max_coin_qty: NonZeroU64,
    max_native_pc_qty_including_fees: NonZeroU64,
    self_trade_behavior: SelfTradeBehavior,
    order_type: OrderType,
    client_order_id: u64,
    limit: u16,
) -> Result<()> {
    let referral = ctx.remaining_accounts.get(0);
    let ix = serum_dex::instruction::new_order(
        ctx.accounts.market.key,
        ctx.accounts.open_orders.key,
        ctx.accounts.request_queue.key,
        ctx.accounts.event_queue.key,
        ctx.accounts.market_bids.key,
        ctx.accounts.market_asks.key,
        ctx.accounts.order_payer_token_account.key,
        ctx.accounts.open_orders_authority.key,
        ctx.accounts.coin_vault.key,
        ctx.accounts.pc_vault.key,
        ctx.accounts.token_program.key,
        ctx.accounts.rent.key,
        referral.map(|r| r.key),
        &ID,
        side,
        limit_price,
        max_coin_qty,
        order_type,
        client_order_id,
        self_trade_behavior,
        limit,
        max_native_pc_qty_including_fees,
    )
    .map_err(|pe| ProgramError::from(pe))?;
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;
    Ok(())
}

pub fn cancel_order_v2<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, CancelOrderV2<'info>>,
    side: Side,
    order_id: u128,
) -> Result<()> {
    let ix = serum_dex::instruction::cancel_order(
        &ID,
        ctx.accounts.market.key,
        ctx.accounts.market_bids.key,
        ctx.accounts.market_asks.key,
        ctx.accounts.open_orders.key,
        ctx.accounts.open_orders_authority.key,
        ctx.accounts.event_queue.key,
        side,
        order_id,
    )
    .map_err(|pe| ProgramError::from(pe))?;
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;
    Ok(())
}

pub fn settle_funds<'info>(ctx: CpiContext<'_, '_, '_, 'info, SettleFunds<'info>>) -> Result<()> {
    let referral = ctx.remaining_accounts.get(0);
    let ix = serum_dex::instruction::settle_funds(
        &ID,
        ctx.accounts.market.key,
        ctx.accounts.token_program.key,
        ctx.accounts.open_orders.key,
        ctx.accounts.open_orders_authority.key,
        ctx.accounts.coin_vault.key,
        ctx.accounts.coin_wallet.key,
        ctx.accounts.pc_vault.key,
        ctx.accounts.pc_wallet.key,
        referral.map(|r| r.key),
        ctx.accounts.vault_signer.key,
    )
    .map_err(|pe| ProgramError::from(pe))?;
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;
    Ok(())
}

pub fn init_open_orders<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, InitOpenOrders<'info>>,
) -> Result<()> {
    let ix = serum_dex::instruction::init_open_orders(
        &ID,
        ctx.accounts.open_orders.key,
        ctx.accounts.authority.key,
        ctx.accounts.market.key,
        ctx.remaining_accounts.first().map(|acc| acc.key),
    )
    .map_err(|pe| ProgramError::from(pe))?;
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;
    Ok(())
}

pub fn close_open_orders<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, CloseOpenOrders<'info>>,
) -> Result<()> {
    let ix = serum_dex::instruction::close_open_orders(
        &ID,
        ctx.accounts.open_orders.key,
        ctx.accounts.authority.key,
        ctx.accounts.destination.key,
        ctx.accounts.market.key,
    )
    .map_err(|pe| ProgramError::from(pe))?;
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;
    Ok(())
}

pub fn sweep_fees<'info>(ctx: CpiContext<'_, '_, '_, 'info, SweepFees<'info>>) -> Result<()> {
    let ix = serum_dex::instruction::sweep_fees(
        &ID,
        ctx.accounts.market.key,
        ctx.accounts.pc_vault.key,
        ctx.accounts.sweep_authority.key,
        ctx.accounts.sweep_receiver.key,
        ctx.accounts.vault_signer.key,
        ctx.accounts.token_program.key,
    )
    .map_err(|pe| ProgramError::from(pe))?;
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;
    Ok(())
}

pub fn initialize_market<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, InitializeMarket<'info>>,
    coin_lot_size: u64,
    pc_lot_size: u64,
    vault_signer_nonce: u64,
    pc_dust_threshold: u64,
) -> Result<()> {
    let authority = ctx.remaining_accounts.get(0);
    let prune_authority = ctx.remaining_accounts.get(1);
    let ix = serum_dex::instruction::initialize_market(
        ctx.accounts.market.key,
        &ID,
        ctx.accounts.coin_mint.key,
        ctx.accounts.pc_mint.key,
        ctx.accounts.coin_vault.key,
        ctx.accounts.pc_vault.key,
        authority.map(|r| r.key),
        prune_authority.map(|r| r.key),
        ctx.accounts.bids.key,
        ctx.accounts.asks.key,
        ctx.accounts.req_q.key,
        ctx.accounts.event_q.key,
        coin_lot_size,
        pc_lot_size,
        vault_signer_nonce,
        pc_dust_threshold,
    )
    .map_err(|pe| ProgramError::from(pe))?;
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;
    Ok(())
}

#[derive(Accounts)]
pub struct NewOrderV3<'info> {
    pub market: AccountInfo<'info>,
    pub open_orders: AccountInfo<'info>,
    pub request_queue: AccountInfo<'info>,
    pub event_queue: AccountInfo<'info>,
    pub market_bids: AccountInfo<'info>,
    pub market_asks: AccountInfo<'info>,
    // Token account where funds are transferred from for the order. If
    // posting a bid market A/B, then this is the SPL token account for B.
    pub order_payer_token_account: AccountInfo<'info>,
    pub open_orders_authority: AccountInfo<'info>,
    // Also known as the "base" currency. For a given A/B market,
    // this is the vault for the A mint.
    pub coin_vault: AccountInfo<'info>,
    // Also known as the "quote" currency. For a given A/B market,
    // this is the vault for the B mint.
    pub pc_vault: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CancelOrderV2<'info> {
    pub market: AccountInfo<'info>,
    pub market_bids: AccountInfo<'info>,
    pub market_asks: AccountInfo<'info>,
    pub open_orders: AccountInfo<'info>,
    pub open_orders_authority: AccountInfo<'info>,
    pub event_queue: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SettleFunds<'info> {
    pub market: AccountInfo<'info>,
    pub open_orders: AccountInfo<'info>,
    pub open_orders_authority: AccountInfo<'info>,
    pub coin_vault: AccountInfo<'info>,
    pub pc_vault: AccountInfo<'info>,
    pub coin_wallet: AccountInfo<'info>,
    pub pc_wallet: AccountInfo<'info>,
    pub vault_signer: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
}

/// To use an (optional) market authority, add it as the first account of the
/// CpiContext's `remaining_accounts` Vec.
#[derive(Accounts)]
pub struct InitOpenOrders<'info> {
    pub open_orders: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
    pub market: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CloseOpenOrders<'info> {
    pub open_orders: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
    pub destination: AccountInfo<'info>,
    pub market: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SweepFees<'info> {
    pub market: AccountInfo<'info>,
    pub pc_vault: AccountInfo<'info>,
    pub sweep_authority: AccountInfo<'info>,
    pub sweep_receiver: AccountInfo<'info>,
    pub vault_signer: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct InitializeMarket<'info> {
    pub market: AccountInfo<'info>,
    pub coin_mint: AccountInfo<'info>,
    pub pc_mint: AccountInfo<'info>,
    pub coin_vault: AccountInfo<'info>,
    pub pc_vault: AccountInfo<'info>,
    pub bids: AccountInfo<'info>,
    pub asks: AccountInfo<'info>,
    pub req_q: AccountInfo<'info>,
    pub event_q: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
}

#[derive(Clone)]
pub struct Dex;

impl anchor_lang::Id for Dex {
    fn id() -> Pubkey {
        ID
    }
}
