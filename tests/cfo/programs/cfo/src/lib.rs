// WIP. This program has been checkpointed and is not production ready.

use anchor_lang::prelude::*;
use anchor_lang::pinocchio_runtime::sysvar_instructions as tx_instructions;
use anchor_spl::dex::{self, Dex};
use anchor_spl::token::{self, Mint, Token, TokenAccount};
use lockup::program::Lockup;
use registry::program::Registry;
use registry::{Registrar, RewardVendorKind};
use serum_dex::state::OpenOrders;
use std::convert::TryInto;
use std::mem::size_of;
use swap::program::Swap;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

/// CFO is the program representing the Serum chief financial officer. It is
/// the program responsible for collecting and distributing fees from the Serum
/// DEX.
#[program]
pub mod cfo {
    use super::*;

    /// Creates a financial officer account associated with a DEX program ID.
    #[access_control(is_distribution_valid(&d))]
    pub fn create_officer(
        ctx: Context<CreateOfficer>,
        bumps: OfficerBumps,
        d: Distribution,
        registrar: Pubkey,
        msrm_registrar: Pubkey,
    ) -> Result<()> {
        let officer = &mut ctx.accounts.officer;
        officer.authority = ctx.accounts.authority.key();
        officer.swap_program = ctx.accounts.swap_program.key();
        officer.dex_program = ctx.accounts.dex_program.key();
        officer.distribution = d;
        officer.registrar = registrar;
        officer.msrm_registrar = msrm_registrar;
        officer.stake = ctx.accounts.stake.key();
        officer.treasury = ctx.accounts.treasury.key();
        officer.srm_vault = ctx.accounts.srm_vault.key();
        officer.bumps = bumps;
        emit!(OfficerDidCreate {
            pubkey: officer.key(),
        });
        Ok(())
    }

    /// Creates a market authorization token.
    pub fn authorize_market(ctx: Context<AuthorizeMarket>, bump: u8) -> Result<()> {
        ctx.accounts.market_auth.bump = bump;
        Ok(())
    }

    /// Revokes a market authorization token.
    pub fn revoke_market(_ctx: Context<RevokeMarket>) -> Result<()> {
        Ok(())
    }

    /// Creates a deterministic token account owned by the CFO.
    /// This should be used when a new mint is used for collecting fees.
    /// Can only be called once per token CFO and token mint.
    pub fn create_officer_token(_ctx: Context<CreateOfficerToken>, _bump: u8) -> Result<()> {
        Ok(())
    }

    /// Creates an open orders account for the given market.
    pub fn create_officer_open_orders(
        ctx: Context<CreateOfficerOpenOrders>,
        _bump: u8,
    ) -> Result<()> {
        let seeds = [
            ctx.accounts.dex_program.key().as_ref(),
            &[ctx.accounts.officer.bumps.bump],
        ];
        let cpi_ctx = CpiContext::from(&*ctx.accounts);
        dex::init_open_orders(cpi_ctx.with_signer(&[&seeds])).map_err(Into::into)
    }

    /// Updates the cfo's fee distribution.
    #[access_control(is_distribution_valid(&d))]
    pub fn set_distribution(ctx: Context<SetDistribution>, d: Distribution) -> Result<()> {
        ctx.accounts.officer.distribution = d.clone();
        emit!(DistributionDidChange { distribution: d });
        Ok(())
    }

    /// Transfers fees from the dex to the CFO.
    pub fn sweep_fees(ctx: Context<SweepFees>) -> Result<()> {
        let cpi_ctx = CpiContext::from(&*ctx.accounts);
        let seeds = [
            ctx.accounts.dex.dex_program.key().as_ref(),
            &[ctx.accounts.officer.bumps.bump],
        ];
        dex::sweep_fees(cpi_ctx.with_signer(&[&seeds])).map_err(Into::into)
    }

    /// Convert the CFO's entire non-SRM token balance into USDC.
    /// Assumes USDC is the quote currency.
    #[access_control(is_not_trading(&ctx.accounts.instructions))]
    pub fn swap_to_usdc(ctx: Context<SwapToUsdc>, min_exchange_rate: ExchangeRate) -> Result<()> {
        let seeds = [
            ctx.accounts.dex_program.key().as_ref(),
            &[ctx.accounts.officer.bumps.bump],
        ];
        let cpi_ctx = CpiContext::from(&*ctx.accounts);
        swap::cpi::swap(
            cpi_ctx.with_signer(&[&seeds]),
            swap::Side::Ask,
            ctx.accounts.from_vault.amount,
            min_exchange_rate.into(),
        )
        .map_err(Into::into)
    }

    /// Convert the CFO's entire token balance into SRM.
    /// Assumes SRM is the base currency.
    #[access_control(is_not_trading(&ctx.accounts.instructions))]
    pub fn swap_to_srm(ctx: Context<SwapToSrm>, min_exchange_rate: ExchangeRate) -> Result<()> {
        let seeds = [
            ctx.accounts.dex_program.key().as_ref(),
            &[ctx.accounts.officer.bumps.bump],
        ];
        let cpi_ctx = CpiContext::from(&*ctx.accounts);
        swap::cpi::swap(
            cpi_ctx.with_signer(&[&seeds]),
            swap::Side::Bid,
            ctx.accounts.usdc_vault.amount,
            min_exchange_rate.into(),
        )
        .map_err(Into::into)
    }

    /// Distributes srm tokens to the various categories. Before calling this,
    /// one must convert the fees into SRM via the swap APIs.
    #[access_control(is_distribution_ready(&ctx.accounts))]
    pub fn distribute(ctx: Context<Distribute>) -> Result<()> {
        let total_fees = ctx.accounts.srm_vault.amount;
        let seeds = [
            ctx.accounts.dex_program.key().as_ref(),
            &[ctx.accounts.officer.bumps.bump],
        ];

        // Burn.
        let burn_amount: u64 = u128::from(total_fees)
            .checked_mul(ctx.accounts.officer.distribution.burn.into())
            .unwrap()
            .checked_div(100)
            .unwrap()
            .try_into()
            .map_err(|_| error!(ErrorCode::U128CannotConvert))?;
        token::burn(ctx.accounts.into_burn().with_signer(&[&seeds]), burn_amount)?;

        // Stake.
        let stake_amount: u64 = u128::from(total_fees)
            .checked_mul(ctx.accounts.officer.distribution.stake.into())
            .unwrap()
            .checked_div(100)
            .unwrap()
            .try_into()
            .map_err(|_| error!(ErrorCode::U128CannotConvert))?;
        token::transfer(
            ctx.accounts.into_stake_transfer().with_signer(&[&seeds]),
            stake_amount,
        )?;

        // Treasury.
        let treasury_amount: u64 = u128::from(total_fees)
            .checked_mul(ctx.accounts.officer.distribution.treasury.into())
            .unwrap()
            .checked_div(100)
            .unwrap()
            .try_into()
            .map_err(|_| error!(ErrorCode::U128CannotConvert))?;
        token::transfer(
            ctx.accounts.into_treasury_transfer().with_signer(&[&seeds]),
            treasury_amount,
        )?;

        Ok(())
    }

    #[access_control(is_stake_reward_ready(&ctx.accounts))]
    pub fn drop_stake_reward(ctx: Context<DropStakeReward>) -> Result<()> {
        // Common reward parameters.
        let expiry_ts = 1853942400; // 9/30/2028.
        let expiry_receiver = ctx.accounts.officer.key();
        let locked_kind = {
            let start_ts = 1633017600; // 9/30/2021.
            let end_ts = 1822320000; // 9/30/2027.
            let period_count = 2191;
            RewardVendorKind::Locked {
                start_ts,
                end_ts,
                period_count,
            }
        };
        let seeds = [
            ctx.accounts.dex_program.key().as_ref(),
            &[ctx.accounts.officer.bumps.bump],
        ];

        // Total amount staked denominated in SRM (i.e. MSRM is converted to
        // SRM)
        let total_pool_value = u128::from(ctx.accounts.srm.pool_mint.supply)
            .checked_mul(500)
            .unwrap()
            .checked_add(
                u128::from(ctx.accounts.msrm.pool_mint.supply)
                    .checked_mul(1_000_000)
                    .unwrap(),
            )
            .unwrap();

        // Total reward split between both the SRM and MSRM stake pools.
        let total_reward_amount = u128::from(ctx.accounts.stake.amount);

        // Proportion of the reward going to the srm pool.
        //
        // total_reward_amount * (srm_pool_value / total_pool_value)
        //
        let srm_amount: u64 = u128::from(ctx.accounts.srm.pool_mint.supply)
            .checked_mul(500)
            .unwrap()
            .checked_mul(total_reward_amount)
            .unwrap()
            .checked_div(total_pool_value)
            .unwrap()
            .try_into()
            .map_err(|_| error!(ErrorCode::U128CannotConvert))?;

        // Proportion of the reward going to the msrm pool.
        //
        // total_reward_amount * (msrm_pool_value / total_pool_value)
        //
        let msrm_amount = u128::from(ctx.accounts.msrm.pool_mint.supply)
            .checked_mul(total_reward_amount)
            .unwrap()
            .checked_div(total_pool_value)
            .unwrap()
            .try_into()
            .map_err(|_| error!(ErrorCode::U128CannotConvert))?;

        // SRM drop.
        {
            // Drop locked reward.
            let (_, nonce) = Pubkey::find_program_address(
                &[
                    ctx.accounts.srm.registrar.key().as_ref(),
                    ctx.accounts.srm.vendor.key().as_ref(),
                ],
                &ctx.accounts.token_program.key(),
            );
            registry::cpi::drop_reward(
                ctx.accounts.into_srm_reward().with_signer(&[&seeds[..]]),
                locked_kind.clone(),
                srm_amount.try_into().unwrap(),
                expiry_ts,
                expiry_receiver,
                nonce,
            )?;

            // Drop unlocked reward.
            registry::cpi::drop_reward(
                ctx.accounts.into_srm_reward().with_signer(&[&seeds[..]]),
                RewardVendorKind::Unlocked,
                srm_amount,
                expiry_ts,
                expiry_receiver,
                nonce,
            )?;
        }

        // MSRM drop.
        {
            // Drop locked reward.
            let (_, nonce) = Pubkey::find_program_address(
                &[
                    ctx.accounts.msrm.registrar.key().as_ref(),
                    ctx.accounts.msrm.vendor.key().as_ref(),
                ],
                &ctx.accounts.token_program.key(),
            );
            registry::cpi::drop_reward(
                ctx.accounts.into_msrm_reward().with_signer(&[&seeds[..]]),
                locked_kind,
                msrm_amount,
                expiry_ts,
                expiry_receiver,
                nonce,
            )?;

            // Drop unlocked reward.
            registry::cpi::drop_reward(
                ctx.accounts.into_msrm_reward().with_signer(&[&seeds[..]]),
                RewardVendorKind::Unlocked,
                msrm_amount,
                expiry_ts,
                expiry_receiver,
                nonce,
            )?;
        }

        Ok(())
    }
}

// Context accounts.

#[derive(Accounts)]
#[instruction(bumps: OfficerBumps)]
pub struct CreateOfficer {
    #[account(
        init,
        seeds = [dex_program.key().as_ref()],
        bump,
        payer = authority,
        space = Officer::LEN + 8
    )]
    officer: Box<Account<Officer>>,
    #[account(
        init,
        seeds = [b"token", officer.key().as_ref(), srm_mint.key().as_ref()],
        bump,
        payer = authority,
        token::mint = srm_mint,
        token::authority = officer
    )]
    srm_vault: Box<Account<TokenAccount>>,
    #[account(
        init,
        seeds = [b"token", officer.key().as_ref(), usdc_mint.key().as_ref()],
        bump,
        payer = authority,
        token::mint = usdc_mint,
        token::authority = officer
    )]
    usdc_vault: Box<Account<TokenAccount>>,
    #[account(
        init,
        seeds = [b"stake", officer.key().as_ref()],
        bump,
        payer = authority,
        token::mint = srm_mint,
        token::authority = officer
    )]
    stake: Box<Account<TokenAccount>>,
    #[account(
        init,
        seeds = [b"treasury", officer.key().as_ref()],
        bump,
        payer = authority,
        token::mint = srm_mint,
        token::authority = officer
    )]
    treasury: Box<Account<TokenAccount>>,
    #[account(mut)]
    authority: Signer,
    #[cfg_attr(
        not(feature = "test"),
        account(address = mint::SRM),
    )]
    srm_mint: Box<Account<Mint>>,
    #[cfg_attr(
        not(feature = "test"),
        account(address = mint::USDC),
    )]
    usdc_mint: Box<Account<Mint>>,
    dex_program: Program<Dex>,
    swap_program: Program<Swap>,
    system_program: Program<System>,
    token_program: Program<Token>,
    rent: Sysvar<Rent>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct AuthorizeMarket {
    #[account(has_one = authority)]
    officer: Account<Officer>,
    authority: Signer,
    #[account(
        init,
        payer = payer,
        seeds = [b"market-auth", officer.key().as_ref(), market.key().as_ref()],
        bump,
        space = MarketAuth::LEN + 8
    )]
    market_auth: Account<MarketAuth>,
    #[account(mut)]
    payer: Signer,
    // Not read or written to so not validated.
    market: UncheckedAccount,
    system_program: Program<System>,
}

#[derive(Accounts)]
pub struct RevokeMarket {
    #[account(has_one = authority)]
    pub officer: Account<Officer>,
    pub authority: Signer,
    #[account(mut, close = payer)]
    pub auth: Account<MarketAuth>,
    pub payer: Signer,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct CreateOfficerToken {
    officer: Account<Officer>,
    #[account(
        init,
        seeds = [b"token", officer.key().as_ref(), mint.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = officer,
        payer = payer
    )]
    token: Account<TokenAccount>,
    mint: Account<Mint>,
    #[account(mut)]
    payer: Signer,
    system_program: Program<System>,
    token_program: Program<Token>,
    rent: Sysvar<Rent>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct CreateOfficerOpenOrders {
    officer: Account<Officer>,
    #[account(
        init,
        seeds = [b"open-orders", officer.key().as_ref(), market.key().as_ref()],
        bump,
        space = 12 + size_of::<OpenOrders>(),
        payer = payer,
        owner = dex::ID,
    )]
    open_orders: UncheckedAccount,
    #[account(mut)]
    payer: Signer,
    dex_program: Program<Dex>,
    system_program: Program<System>,
    rent: Sysvar<Rent>,
    // Used for CPI. Not read or written so not validated.
    market: UncheckedAccount,
}

#[derive(Accounts)]
pub struct SetDistribution {
    #[account(has_one = authority)]
    officer: Account<Officer>,
    authority: Signer,
}

#[derive(Accounts)]
pub struct SweepFees {
    #[account(
        seeds = [dex.dex_program.key().as_ref()],
        bump = officer.bumps.bump,
    )]
    officer: Account<Officer>,
    #[account(
        mut,
        seeds = [b"token", officer.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    sweep_vault: Account<TokenAccount>,
    mint: Account<Mint>,
    dex: DexAccounts,
}

// DexAccounts are safe because they are used for CPI only.
// They are not read or written and so are not validated.
#[derive(Accounts)]
pub struct DexAccounts {
    #[account(mut)]
    market: UncheckedAccount,
    #[account(mut)]
    pc_vault: UncheckedAccount,
    sweep_authority: UncheckedAccount,
    vault_signer: UncheckedAccount,
    dex_program: Program<Dex>,
    token_program: Program<Token>,
}

#[derive(Accounts)]
pub struct SwapToUsdc {
    #[account(
        seeds = [dex_program.key().as_ref()],
        bump = officer.bumps.bump,
    )]
    officer: Box<Account<Officer>>,
    market: DexMarketAccounts,
    #[account(
        seeds = [b"market-auth", officer.key().as_ref(), market.market.key().as_ref()],
        bump = market_auth.bump,
    )]
    market_auth: Account<MarketAuth>,
    #[account(
        mut,
        constraint = &officer.treasury != &from_vault.key(),
        constraint = &officer.stake != &from_vault.key(),
    )]
    from_vault: Box<Account<TokenAccount>>,
    #[account(
        mut,
        seeds = [b"token", officer.key().as_ref(), usdc_mint.key().as_ref()],
        bump,
    )]
    usdc_vault: Box<Account<TokenAccount>>,
    #[cfg_attr(not(feature = "test"), account(address = mint::USDC))]
    usdc_mint: Box<Account<Mint>>,
    swap_program: Program<Swap>,
    dex_program: Program<Dex>,
    token_program: Program<Token>,
    #[account(address = tx_instructions::INSTRUCTIONS_ID)]
    instructions: UncheckedAccount,
    rent: Sysvar<Rent>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct SwapToSrm {
    #[account(
        seeds = [dex_program.key().as_ref()],
        bump = officer.bumps.bump,
    )]
    officer: Box<Account<Officer>>,
    market: DexMarketAccounts,
    #[account(
        seeds = [b"market-auth", officer.key().as_ref(), market.market.key().as_ref()],
        bump = market_auth.bump,
    )]
    market_auth: Account<MarketAuth>,
    #[account(
        mut,
        seeds = [b"token", officer.key().as_ref(), usdc_mint.key().as_ref()],
        bump,
    )]
    usdc_vault: Box<Account<TokenAccount>>,
    #[account(
        mut,
        seeds = [b"token", officer.key().as_ref(), srm_mint.key().as_ref()],
        bump,
    )]
    srm_vault: Box<Account<TokenAccount>>,
    #[cfg_attr(not(feature = "test"), account(address = mint::SRM))]
    srm_mint: Box<Account<Mint>>,
    #[cfg_attr(not(feature = "test"), account(address = mint::USDC))]
    usdc_mint: Box<Account<Mint>>,
    swap_program: Program<Swap>,
    dex_program: Program<Dex>,
    token_program: Program<Token>,
    #[account(address = tx_instructions::INSTRUCTIONS_ID)]
    instructions: UncheckedAccount,
    rent: Sysvar<Rent>,
}

// Dex accounts are used for CPI only.
// They are not read or written and so are not validated.
#[derive(Accounts)]
pub struct DexMarketAccounts {
    #[account(mut)]
    market: UncheckedAccount,
    #[account(mut)]
    open_orders: UncheckedAccount,
    #[account(mut)]
    request_queue: UncheckedAccount,
    #[account(mut)]
    event_queue: UncheckedAccount,
    #[account(mut)]
    bids: UncheckedAccount,
    #[account(mut)]
    asks: UncheckedAccount,
    // The `spl_token::Account` that funds will be taken from, i.e., transferred
    // from the user into the market's vault.
    //
    // For bids, this is the base currency. For asks, the quote.
    #[account(mut)]
    order_payer_token_account: UncheckedAccount,
    // Also known as the "base" currency. For a given A/B market,
    // this is the vault for the A mint.
    #[account(mut)]
    coin_vault: UncheckedAccount,
    // Also known as the "quote" currency. For a given A/B market,
    // this is the vault for the B mint.
    #[account(mut)]
    pc_vault: UncheckedAccount,
    // PDA owner of the DEX's token accounts for base + quote currencies.
    vault_signer: UncheckedAccount,
}

#[derive(Accounts)]
pub struct Distribute {
    #[account(
        has_one = srm_vault,
        has_one = treasury,
        has_one = stake,
    )]
    officer: Box<Account<Officer>>,
    #[account(mut)]
    treasury: Box<Account<TokenAccount>>,
    #[account(mut)]
    stake: Account<TokenAccount>,
    #[account(mut)]
    srm_vault: Account<TokenAccount>,
    #[account(mut)]
    srm_mint: Account<Mint>,
    token_program: Program<Token>,
    dex_program: Program<Dex>,
}

#[derive(Accounts)]
pub struct DropStakeReward {
    #[account(
        has_one = stake,
        constraint = srm.registrar.key() == &officer.registrar,
        constraint = msrm.registrar.key() == &officer.msrm_registrar,
    )]
    officer: Box<Account<Officer>>,
    #[account(
        seeds = [b"stake", officer.key().as_ref()],
        bump = officer.bumps.stake,
    )]
    stake: Box<Account<TokenAccount>>,
    #[cfg_attr(
        not(feature = "test"),
        account(address = mint::SRM),
    )]
    mint: UncheckedAccount,
    srm: DropStakeRewardPool,
    msrm: DropStakeRewardPool,
    msrm_registrar: Box<Account<Registrar>>,
    token_program: Program<Token>,
    registry_program: Program<Registry>,
    lockup_program: Program<Lockup>,
    dex_program: Program<Dex>,
    clock: Sysvar<Clock>,
    rent: Sysvar<Rent>,
}

// Don't bother doing validation on the individual accounts. Allow the stake
// program to handle it.
#[derive(Accounts)]
pub struct DropStakeRewardPool {
    registrar: UncheckedAccount,
    reward_event_q: UncheckedAccount,
    pool_mint: Account<Mint>,
    vendor: UncheckedAccount,
    vendor_vault: UncheckedAccount,
}

// Accounts.

/// Officer represents a deployed instance of the CFO mechanism. It is tied
/// to a single deployment of the dex program.
///
/// PDA - [dex_program_id].
#[account]
pub struct Officer {
    // Privileged account.
    pub authority: Pubkey, // 32
    // Vault holding the officer's SRM tokens prior to distribution.
    pub srm_vault: Pubkey, // 32
    // Escrow SRM vault holding tokens which are dropped onto stakers.
    pub stake: Pubkey, // 32
    // SRM token account to send treasury earned tokens to.
    pub treasury: Pubkey, // 32
    // Defines the fee distribution, i.e., what percent each fee category gets.
    pub distribution: Distribution, // Distribution::LEN
    // Swap frontend for the dex.
    pub swap_program: Pubkey, // 32
    // Dex program the officer is associated with.
    pub dex_program: Pubkey, // 32
    // SRM stake pool address
    pub registrar: Pubkey, // 32
    // MSRM stake pool address.
    pub msrm_registrar: Pubkey, // 32
    // Bump seeds for pdas.
    pub bumps: OfficerBumps, // OfficerBumps::LEN
}

impl Officer {
    pub const LEN: usize = 8 * 32 + Distribution::LEN + OfficerBumps::LEN;
}

/// MarketAuth represents an authorization token created by the Officer
/// authority. This is used as an authorization token which allows one to
/// permissionlessly invoke the swap instructions on the market. Without this
/// one would be able to create their own market with prices unfavorable
/// to the smart contract (and subsequently swap on it).
///
/// Because a PDA is used here, the account existing (without a tombstone) is
/// proof of the validity of a given market, which means that anyone can use
/// the vault here to swap.
///
/// PDA - [b"market-auth", officer, market_address]
#[account]
pub struct MarketAuth {
    // Bump seed for this account's PDA.
    pub bump: u8, // 1
}

impl MarketAuth {
    pub const LEN: usize = 1;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct OfficerBumps {
    pub bump: u8,     // 1
    pub srm: u8,      // 1
    pub usdc: u8,     // 1
    pub stake: u8,    // 1
    pub treasury: u8, // 1
}

impl OfficerBumps {
    pub const LEN: usize = 5;
}

#[derive(AnchorSerialize, AnchorDeserialize, Default, Clone)]
pub struct Distribution {
    burn: u8,     // 1
    stake: u8,    // 1
    treasury: u8, // 1
}

impl Distribution {
    pub const LEN: usize = 3;
}

// CpiContext transformations.

impl From<&CreateOfficerOpenOrders>
    for CpiContext<'_, '_, dex::InitOpenOrders>
{
    fn from(accs: &CreateOfficerOpenOrders) -> Self {
        let program = accs.dex_program.key();
        let accounts = dex::InitOpenOrders {
            open_orders: accs.open_orders.to_account_info(),
            authority: accs.officer.to_account_info(),
            market: accs.market.to_account_info(),
            rent: accs.rent.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

impl From<&SweepFees> for CpiContext<'_, '_, dex::SweepFees> {
    fn from(sweep: &SweepFees) -> Self {
        let program = sweep.dex.dex_program.key();
        let accounts = dex::SweepFees {
            market: sweep.dex.market.to_account_info(),
            pc_vault: sweep.dex.pc_vault.to_account_info(),
            sweep_authority: sweep.dex.sweep_authority.to_account_info(),
            sweep_receiver: sweep.sweep_vault.to_account_info(),
            vault_signer: sweep.dex.vault_signer.to_account_info(),
            token_program: sweep.dex.token_program.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

impl From<&SwapToSrm>
    for CpiContext<'_, '_, swap::cpi::accounts::Swap>
{
    fn from(accs: &SwapToSrm) -> Self {
        let program = accs.swap_program.key();
        let accounts = swap::cpi::accounts::Swap {
            market: swap::cpi::accounts::MarketAccounts {
                market: accs.market.market.to_account_info(),
                open_orders: accs.market.open_orders.to_account_info(),
                request_queue: accs.market.request_queue.to_account_info(),
                event_queue: accs.market.event_queue.to_account_info(),
                bids: accs.market.bids.to_account_info(),
                asks: accs.market.asks.to_account_info(),
                order_payer_token_account: accs.market.order_payer_token_account.to_account_info(),
                coin_vault: accs.market.coin_vault.to_account_info(),
                pc_vault: accs.market.pc_vault.to_account_info(),
                vault_signer: accs.market.vault_signer.to_account_info(),
                coin_wallet: accs.srm_vault.to_account_info(),
            },
            authority: accs.officer.to_account_info(),
            pc_wallet: accs.usdc_vault.to_account_info(),
            dex_program: accs.dex_program.to_account_info(),
            token_program: accs.token_program.to_account_info(),
            rent: accs.rent.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

impl From<&SwapToUsdc>
    for CpiContext<'_, '_, swap::cpi::accounts::Swap>
{
    fn from(accs: &SwapToUsdc) -> Self {
        let program = accs.swap_program.key();
        let accounts = swap::cpi::accounts::Swap {
            market: swap::cpi::accounts::MarketAccounts {
                market: accs.market.market.to_account_info(),
                open_orders: accs.market.open_orders.to_account_info(),
                request_queue: accs.market.request_queue.to_account_info(),
                event_queue: accs.market.event_queue.to_account_info(),
                bids: accs.market.bids.to_account_info(),
                asks: accs.market.asks.to_account_info(),
                order_payer_token_account: accs.market.order_payer_token_account.to_account_info(),
                coin_vault: accs.market.coin_vault.to_account_info(),
                pc_vault: accs.market.pc_vault.to_account_info(),
                vault_signer: accs.market.vault_signer.to_account_info(),
                coin_wallet: accs.from_vault.to_account_info(),
            },
            authority: accs.officer.to_account_info(),
            pc_wallet: accs.usdc_vault.to_account_info(),
            dex_program: accs.dex_program.to_account_info(),
            token_program: accs.token_program.to_account_info(),
            rent: accs.rent.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

impl Distribute {
    fn into_burn(&self) -> CpiContext<'_, '_, token::Burn> {
        let program_id = self.token_program.key();
        let accounts = token::Burn {
            mint: self.srm_mint.to_account_info(),
            from: self.srm_vault.to_account_info(),
            authority: self.officer.to_account_info(),
        };
        CpiContext::new(program_id, accounts)
    }

    fn into_stake_transfer(&self) -> CpiContext<'_, '_, token::Transfer> {
        let program_id = self.token_program.key();
        let accounts = token::Transfer {
            from: self.srm_vault.to_account_info(),
            to: self.stake.to_account_info(),
            authority: self.officer.to_account_info(),
        };
        CpiContext::new(program_id, accounts)
    }

    fn into_treasury_transfer(&self) -> CpiContext<'_, '_, token::Transfer> {
        let program_id = self.token_program.key();
        let accounts = token::Transfer {
            from: self.srm_vault.to_account_info(),
            to: self.treasury.to_account_info(),
            authority: self.officer.to_account_info(),
        };
        CpiContext::new(program_id, accounts)
    }
}

impl DropStakeReward {
    fn into_srm_reward(
        &self,
    ) -> CpiContext<'_, '_, registry::cpi::accounts::DropReward> {
        let program_id = self.registry_program.key();
        let accounts = registry::cpi::accounts::DropReward {
            registrar: self.srm.registrar.to_account_info(),
            reward_event_q: self.srm.reward_event_q.to_account_info(),
            pool_mint: self.srm.pool_mint.to_account_info(),
            vendor: self.srm.vendor.to_account_info(),
            vendor_vault: self.srm.vendor_vault.to_account_info(),
            depositor: self.stake.to_account_info(),
            depositor_authority: self.officer.to_account_info(),
            token_program: self.token_program.to_account_info(),
            clock: self.clock.to_account_info(),
            rent: self.rent.to_account_info(),
        };
        CpiContext::new(program_id, accounts)
    }

    fn into_msrm_reward(
        &self,
    ) -> CpiContext<'_, '_, registry::cpi::accounts::DropReward> {
        let program_id = self.registry_program.key();
        let accounts = registry::cpi::accounts::DropReward {
            registrar: self.msrm.registrar.to_account_info(),
            reward_event_q: self.msrm.reward_event_q.to_account_info(),
            pool_mint: self.msrm.pool_mint.to_account_info(),
            vendor: self.msrm.vendor.to_account_info(),
            vendor_vault: self.msrm.vendor_vault.to_account_info(),
            depositor: self.stake.to_account_info(),
            depositor_authority: self.officer.to_account_info(),
            token_program: self.token_program.to_account_info(),
            clock: self.clock.to_account_info(),
            rent: self.rent.to_account_info(),
        };
        CpiContext::new(program_id, accounts)
    }
}

// Events.

#[event]
pub struct DistributionDidChange {
    distribution: Distribution,
}

#[event]
pub struct OfficerDidCreate {
    pubkey: Pubkey,
}

// Error.

#[error_code]
pub enum ErrorCode {
    #[msg("Distribution does not add to 100")]
    InvalidDistribution,
    #[msg("u128 cannot be converted into u64")]
    U128CannotConvert,
    #[msg("Only one instruction is allowed for this transaction")]
    TooManyInstructions,
    #[msg("Not enough SRM has been accumulated to distribute")]
    InsufficientDistributionAmount,
    #[msg("Must drop more SRM onto the stake pool")]
    InsufficientStakeReward,
}

// Access control.

fn is_distribution_valid(d: &Distribution) -> Result<()> {
    if d.burn + d.stake + d.treasury != 100 {
        return err!(ErrorCode::InvalidDistribution);
    }
    Ok(())
}

fn is_distribution_ready(accounts: &Distribute) -> Result<()> {
    if accounts.srm_vault.amount < 1_000_000 {
        return err!(ErrorCode::InsufficientDistributionAmount);
    }
    Ok(())
}

// `ixs` must be the Instructions sysvar.
fn is_not_trading(ixs: &UncheckedAccount) -> Result<()> {
    let instructions = tx_instructions::Instructions::try_from(ixs.as_ref()).map_err(|_| {
        anchor_lang::error::Error::from(anchor_lang::ProgramError::InvalidAccountData)
    })?;
    match instructions.load_instruction_at(1) {
        Ok(_) => err!(ErrorCode::TooManyInstructions),
        Err(_) => Ok(()),
    }
}

fn is_stake_reward_ready(accounts: &DropStakeReward) -> Result<()> {
    // Min drop is 15,0000 SRM.
    let min_reward: u64 = 15_000_000_000;
    if accounts.stake.amount < min_reward {
        return err!(ErrorCode::InsufficientStakeReward);
    }
    Ok(())
}

// Redefinitions.
//
// The following types are redefined so that they can be parsed into the IDL,
// since Anchor doesn't yet support idl parsing across multiple crates.

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct ExchangeRate {
    rate: u64,
    from_decimals: u8,
    quote_decimals: u8,
    strict: bool,
}

impl From<ExchangeRate> for swap::ExchangeRate {
    fn from(e: ExchangeRate) -> Self {
        let ExchangeRate {
            rate,
            from_decimals,
            quote_decimals,
            strict,
        } = e;
        Self {
            rate,
            from_decimals,
            quote_decimals,
            strict,
        }
    }
}
