use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_lang::solana_program::entrypoint::ProgramResult;
use anchor_lang::{Accounts, CpiContext, ToAccountInfos};
use serum_dex::instruction::SelfTradeBehavior;
use serum_dex::matching::{OrderType, Side};
use std::num::NonZeroU64;

pub use serum_dex;

anchor_lang::solana_program::declare_id!("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin");

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
) -> ProgramResult {
    let referral = ctx.remaining_accounts.iter().next();
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
    )?;
    solana_program::program::invoke_signed(
        &ix,
        &ToAccountInfos::to_account_infos(&ctx),
        ctx.signer_seeds,
    )?;
    Ok(())
}

pub fn settle_funds<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, SettleFunds<'info>>,
) -> ProgramResult {
    let referral = ctx.remaining_accounts.iter().next();
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
    )?;
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
