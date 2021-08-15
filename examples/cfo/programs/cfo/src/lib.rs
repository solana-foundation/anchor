// WIP. This program has been checkpointed and is not production ready.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_program;
use anchor_lang::solana_program::sysvar::instructions as tx_instructions;
use anchor_spl::token::{self, Mint, TokenAccount};
use anchor_spl::{dex, mint};
use registry::{Registrar, RewardVendorKind};
use std::convert::TryInto;

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
        officer.authority = *ctx.accounts.authority.key;
        officer.swap_program = *ctx.accounts.swap_program.key;
        officer.dex_program = *ctx.accounts.dex_program.key;
        officer.distribution = d;
        officer.registrar = registrar;
        officer.msrm_registrar = msrm_registrar;
        officer.stake = *ctx.accounts.stake.to_account_info().key;
        officer.treasury = *ctx.accounts.treasury.to_account_info().key;
        officer.srm_vault = *ctx.accounts.srm_vault.to_account_info().key;
        officer.bumps = bumps;
        emit!(OfficerDidCreate {
            pubkey: *officer.to_account_info().key,
        });
        Ok(())
    }

    /// Creates a deterministic token account owned by the CFO.
    /// This should be used when a new mint is used for collecting fees.
    /// Can only be called once per token CFO and token mint.
    pub fn create_officer_token(_ctx: Context<CreateOfficerToken>, _bump: u8) -> Result<()> {
        Ok(())
    }

    /// Updates the cfo's fee distribution.
    #[access_control(is_distribution_valid(&d))]
    pub fn set_distribution(ctx: Context<SetDistribution>, d: Distribution) -> Result<()> {
        ctx.accounts.officer.distribution = d.clone();
        emit!(DistributionDidChange { distribution: d });
        Ok(())
    }

    /// Transfers fees from the dex to the CFO.
    pub fn sweep_fees<'info>(ctx: Context<'_, '_, '_, 'info, SweepFees<'info>>) -> Result<()> {
        let seeds = [
            ctx.accounts.dex.dex_program.key.as_ref(),
            &[ctx.accounts.officer.bumps.bump],
        ];
        let cpi_ctx = CpiContext::from(&*ctx.accounts);
        dex::sweep_fees(cpi_ctx.with_signer(&[&seeds[..]]))?;
        Ok(())
    }

    /// Convert the CFO's entire non-SRM token balance into USDC.
    /// Assumes USDC is the quote currency.
    #[access_control(is_not_trading(&ctx.accounts.instructions))]
    pub fn swap_to_usdc<'info>(
        ctx: Context<'_, '_, '_, 'info, SwapToUsdc<'info>>,
        min_exchange_rate: ExchangeRate,
    ) -> Result<()> {
        let seeds = [
            ctx.accounts.dex_program.key.as_ref(),
            &[ctx.accounts.officer.bumps.bump],
        ];
        let cpi_ctx = CpiContext::from(&*ctx.accounts);
        swap::cpi::swap(
            cpi_ctx.with_signer(&[&seeds[..]]),
            swap::Side::Bid,
            token::accessor::amount(&ctx.accounts.from_vault)?,
            min_exchange_rate.into(),
        )?;
        Ok(())
    }

    /// Convert the CFO's entire token balance into SRM.
    /// Assumes SRM is the base currency.
    #[access_control(is_not_trading(&ctx.accounts.instructions))]
    pub fn swap_to_srm<'info>(
        ctx: Context<'_, '_, '_, 'info, SwapToSrm<'info>>,
        min_exchange_rate: ExchangeRate,
    ) -> Result<()> {
        let seeds = [
            ctx.accounts.dex_program.key.as_ref(),
            &[ctx.accounts.officer.bumps.bump],
        ];
        let cpi_ctx: CpiContext<'_, '_, '_, 'info, swap::Swap<'info>> = (&*ctx.accounts).into();
        swap::cpi::swap(
            cpi_ctx.with_signer(&[&seeds[..]]),
            swap::Side::Bid,
            token::accessor::amount(&ctx.accounts.from_vault)?,
            min_exchange_rate.into(),
        )?;
        Ok(())
    }

    /// Distributes srm tokens to the various categories. Before calling this,
    /// one must convert the fees into SRM via the swap APIs.
    #[access_control(is_distribution_ready(&ctx.accounts))]
    pub fn distribute<'info>(ctx: Context<'_, '_, '_, 'info, Distribute<'info>>) -> Result<()> {
        let total_fees = ctx.accounts.srm_vault.amount;
        let seeds = [
            ctx.accounts.dex_program.key.as_ref(),
            &[ctx.accounts.officer.bumps.bump],
        ];

        // Burn.
        let burn_amount: u64 = u128::from(total_fees)
            .checked_mul(ctx.accounts.officer.distribution.burn.into())
            .unwrap()
            .checked_div(100)
            .unwrap()
            .try_into()
            .map_err(|_| ErrorCode::U128CannotConvert)?;
        token::burn(
            ctx.accounts.into_burn().with_signer(&[&seeds[..]]),
            burn_amount,
        )?;

        // Stake.
        let stake_amount: u64 = u128::from(total_fees)
            .checked_mul(ctx.accounts.officer.distribution.stake.into())
            .unwrap()
            .checked_div(100)
            .unwrap()
            .try_into()
            .map_err(|_| ErrorCode::U128CannotConvert)?;
        token::transfer(
            ctx.accounts
                .into_stake_transfer()
                .with_signer(&[&seeds[..]]),
            stake_amount,
        )?;

        // Treasury.
        let treasury_amount: u64 = u128::from(total_fees)
            .checked_mul(ctx.accounts.officer.distribution.treasury.into())
            .unwrap()
            .checked_div(100)
            .unwrap()
            .try_into()
            .map_err(|_| ErrorCode::U128CannotConvert)?;
        token::transfer(
            ctx.accounts
                .into_treasury_transfer()
                .with_signer(&[&seeds[..]]),
            treasury_amount,
        )?;

        Ok(())
    }

    #[access_control(is_stake_reward_ready(&ctx.accounts))]
    pub fn drop_stake_reward<'info>(
        ctx: Context<'_, '_, '_, 'info, DropStakeReward<'info>>,
    ) -> Result<()> {
        // Common reward parameters.
        let expiry_ts = 1853942400; // 9/30/2028.
        let expiry_receiver = *ctx.accounts.officer.to_account_info().key;
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
            ctx.accounts.dex_program.key.as_ref(),
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
            .map_err(|_| ErrorCode::U128CannotConvert)?;

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
            .map_err(|_| ErrorCode::U128CannotConvert)?;

        // SRM drop.
        {
            // Drop locked reward.
            let (_, nonce) = Pubkey::find_program_address(
                &[
                    ctx.accounts.srm.registrar.to_account_info().key.as_ref(),
                    ctx.accounts.srm.vendor.to_account_info().key.as_ref(),
                ],
                ctx.accounts.token_program.key,
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
                    ctx.accounts.msrm.registrar.to_account_info().key.as_ref(),
                    ctx.accounts.msrm.vendor.to_account_info().key.as_ref(),
                ],
                ctx.accounts.token_program.key,
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
pub struct CreateOfficer<'info> {
    #[account(
        init,
        seeds = [dex_program.key.as_ref()],
        bump = bumps.bump,
        payer = authority,
    )]
    officer: ProgramAccount<'info, Officer>,
    #[account(
        init,
        seeds = [b"vault", officer.key().as_ref()],
        bump = bumps.srm,
        payer = authority,
        token::mint = mint,
        token::authority = officer,
    )]
    srm_vault: CpiAccount<'info, TokenAccount>,
    #[account(
        init,
        seeds = [b"stake", officer.key().as_ref()],
        bump = bumps.stake,
        payer = authority,
        token::mint = mint,
        token::authority = officer,
    )]
    stake: CpiAccount<'info, TokenAccount>,
    #[account(
        init,
        seeds = [b"treasury", officer.key().as_ref()],
        bump = bumps.treasury,
        payer = authority,
        token::mint = mint,
        token::authority = officer,
    )]
    treasury: CpiAccount<'info, TokenAccount>,
    #[account(signer)]
    authority: AccountInfo<'info>,
    #[cfg_attr(
        not(feature = "test"),
        account(address = mint::SRM),
    )]
    mint: AccountInfo<'info>,
    #[account(executable)]
    dex_program: AccountInfo<'info>,
    #[account(executable)]
    swap_program: AccountInfo<'info>,
    #[account(address = system_program::ID)]
    system_program: AccountInfo<'info>,
    #[account(address = spl_token::ID)]
    token_program: AccountInfo<'info>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct CreateOfficerToken<'info> {
    officer: ProgramAccount<'info, Officer>,
    #[account(
        init,
        seeds = [officer.key().as_ref(), mint.key().as_ref()],
        bump = bump,
        token::mint = mint,
        token::authority = officer,
        payer = payer,
    )]
    token: CpiAccount<'info, TokenAccount>,
    #[account(owner = token_program)]
    mint: AccountInfo<'info>,
    #[account(mut, signer)]
    payer: AccountInfo<'info>,
    #[account(address = system_program::ID)]
    system_program: AccountInfo<'info>,
    #[account(address = spl_token::ID)]
    token_program: AccountInfo<'info>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct SetDistribution<'info> {
    #[account(has_one = authority)]
    officer: ProgramAccount<'info, Officer>,
    #[account(signer)]
    authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SweepFees<'info> {
    #[account(seeds = [dex.dex_program.key.as_ref(), &[officer.bumps.bump]])]
    officer: ProgramAccount<'info, Officer>,
    #[account(
        mut,
        owner = dex.token_program,
        seeds = [officer.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    sweep_vault: CpiAccount<'info, TokenAccount>,
    mint: AccountInfo<'info>,
    dex: Dex<'info>,
}

#[derive(Accounts)]
pub struct Dex<'info> {
    #[account(mut)]
    market: AccountInfo<'info>,
    #[account(mut)]
    pc_vault: AccountInfo<'info>,
    sweep_authority: AccountInfo<'info>,
    vault_signer: AccountInfo<'info>,
    dex_program: AccountInfo<'info>,
    #[account(address = spl_token::ID)]
    token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SwapToUsdc<'info> {
    #[account(seeds = [dex_program.key().as_ref(), &[officer.bumps.bump]])]
    officer: ProgramAccount<'info, Officer>,
    market: DexMarketAccounts<'info>,
    #[account(
        owner = token_program,
        constraint = &officer.treasury != from_vault.key,
        constraint = &officer.stake != from_vault.key,
    )]
    from_vault: AccountInfo<'info>,
    #[account(owner = token_program)]
    quote_vault: AccountInfo<'info>,
    #[account(seeds = [officer.key().as_ref(), mint::USDC.as_ref()], bump)]
    usdc_vault: AccountInfo<'info>,
    #[account(address = swap::ID)]
    swap_program: AccountInfo<'info>,
    #[account(address = dex::ID)]
    dex_program: AccountInfo<'info>,
    #[account(address = token::ID)]
    token_program: AccountInfo<'info>,
    #[account(address = tx_instructions::ID)]
    instructions: AccountInfo<'info>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct SwapToSrm<'info> {
    #[account(seeds = [dex_program.key().as_ref(), &[officer.bumps.bump]])]
    officer: ProgramAccount<'info, Officer>,
    market: DexMarketAccounts<'info>,
    #[account(
        owner = token_program,
        constraint = &officer.treasury != from_vault.key,
        constraint = &officer.stake != from_vault.key,
    )]
    from_vault: AccountInfo<'info>,
    #[account(owner = token_program)]
    quote_vault: AccountInfo<'info>,
    #[account(
        seeds = [officer.key().as_ref(), mint::SRM.as_ref()],
        bump,
        constraint = &officer.treasury != from_vault.key,
        constraint = &officer.stake != from_vault.key,
    )]
    srm_vault: AccountInfo<'info>,
    #[account(address = swap::ID)]
    swap_program: AccountInfo<'info>,
    #[account(address = dex::ID)]
    dex_program: AccountInfo<'info>,
    #[account(address = token::ID)]
    token_program: AccountInfo<'info>,
    #[account(address = tx_instructions::ID)]
    instructions: AccountInfo<'info>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct DexMarketAccounts<'info> {
    #[account(mut)]
    market: AccountInfo<'info>,
    #[account(mut)]
    open_orders: AccountInfo<'info>,
    #[account(mut)]
    request_queue: AccountInfo<'info>,
    #[account(mut)]
    event_queue: AccountInfo<'info>,
    #[account(mut)]
    bids: AccountInfo<'info>,
    #[account(mut)]
    asks: AccountInfo<'info>,
    // The `spl_token::Account` that funds will be taken from, i.e., transferred
    // from the user into the market's vault.
    //
    // For bids, this is the base currency. For asks, the quote.
    #[account(mut)]
    order_payer_token_account: AccountInfo<'info>,
    // Also known as the "base" currency. For a given A/B market,
    // this is the vault for the A mint.
    #[account(mut)]
    coin_vault: AccountInfo<'info>,
    // Also known as the "quote" currency. For a given A/B market,
    // this is the vault for the B mint.
    #[account(mut)]
    pc_vault: AccountInfo<'info>,
    // PDA owner of the DEX's token accounts for base + quote currencies.
    vault_signer: AccountInfo<'info>,
    // User wallets.
    #[account(mut)]
    coin_wallet: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Distribute<'info> {
    #[account(has_one = treasury, has_one = stake)]
    officer: ProgramAccount<'info, Officer>,
    treasury: AccountInfo<'info>,
    stake: AccountInfo<'info>,
    #[account(
        owner = token_program,
        constraint = srm_vault.mint == mint::SRM,
    )]
    srm_vault: CpiAccount<'info, TokenAccount>,
    #[account(address = mint::SRM)]
    mint: AccountInfo<'info>,
    #[account(address = spl_token::ID)]
    token_program: AccountInfo<'info>,
    #[account(address = dex::ID)]
    dex_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct DropStakeReward<'info> {
    #[account(
        has_one = stake,
        constraint = srm.registrar.key == &officer.registrar,
        constraint = msrm.registrar.key == &officer.msrm_registrar,
    )]
    officer: ProgramAccount<'info, Officer>,
    #[account(
        seeds = [b"stake", officer.key().as_ref(), &[officer.bumps.stake]]
    )]
    stake: CpiAccount<'info, TokenAccount>,
    #[cfg_attr(
        not(feature = "test"),
        account(address = mint::SRM),
    )]
    mint: AccountInfo<'info>,
    srm: DropStakeRewardPool<'info>,
    msrm: DropStakeRewardPool<'info>,
    #[account(owner = registry_program)]
    msrm_registrar: CpiAccount<'info, Registrar>,
    #[account(address = token::ID)]
    token_program: AccountInfo<'info>,
    #[account(address = registry::ID)]
    registry_program: AccountInfo<'info>,
    #[account(address = lockup::ID)]
    lockup_program: AccountInfo<'info>,
    #[account(address = dex::ID)]
    dex_program: AccountInfo<'info>,
    clock: Sysvar<'info, Clock>,
    rent: Sysvar<'info, Rent>,
}

// Don't bother doing validation on the individual accounts. Allow the stake
// program to handle it.
#[derive(Accounts)]
pub struct DropStakeRewardPool<'info> {
    registrar: AccountInfo<'info>,
    reward_event_q: AccountInfo<'info>,
    pool_mint: CpiAccount<'info, Mint>,
    vendor: AccountInfo<'info>,
    vendor_vault: AccountInfo<'info>,
}

// Accounts.

#[account]
#[derive(Default)]
pub struct Officer {
    // Priviledged account.
    pub authority: Pubkey,
    // Vault holding the officer's SRM tokens prior to distribution.
    pub srm_vault: Pubkey,
    // Escrow SRM vault holding tokens which are dropped onto stakers.
    pub stake: Pubkey,
    // SRM token account to send treasury earned tokens to.
    pub treasury: Pubkey,
    // Defines the fee distribution, i.e., what percent each fee category gets.
    pub distribution: Distribution,
    // Swap frontend for the dex.
    pub swap_program: Pubkey,
    // Dex program the officer is associated with.
    pub dex_program: Pubkey,
    // SRM stake pool address
    pub registrar: Pubkey,
    // MSRM stake pool address.
    pub msrm_registrar: Pubkey,
    // Bump seeds for pdas.
    pub bumps: OfficerBumps,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct OfficerBumps {
    pub bump: u8,
    pub srm: u8,
    pub stake: u8,
    pub treasury: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Default, Clone)]
pub struct Distribution {
    burn: u8,
    stake: u8,
    treasury: u8,
}

// CpiContext transformations.

impl<'info> From<&SweepFees<'info>> for CpiContext<'_, '_, '_, 'info, dex::SweepFees<'info>> {
    fn from(sweep: &SweepFees<'info>) -> Self {
        let program = sweep.dex.dex_program.to_account_info();
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

impl<'info> From<&SwapToSrm<'info>> for CpiContext<'_, '_, '_, 'info, swap::Swap<'info>> {
    fn from(accs: &SwapToSrm<'info>) -> Self {
        let program = accs.swap_program.to_account_info();
        let accounts = swap::Swap {
            market: swap::MarketAccounts {
                market: accs.market.market.clone(),
                open_orders: accs.market.open_orders.clone(),
                request_queue: accs.market.request_queue.clone(),
                event_queue: accs.market.event_queue.clone(),
                bids: accs.market.bids.clone(),
                asks: accs.market.asks.clone(),
                order_payer_token_account: accs.market.order_payer_token_account.clone(),
                coin_vault: accs.market.coin_vault.clone(),
                pc_vault: accs.market.pc_vault.clone(),
                vault_signer: accs.market.vault_signer.clone(),
                coin_wallet: accs.srm_vault.clone(),
            },
            authority: accs.officer.to_account_info(),
            pc_wallet: accs.from_vault.to_account_info(),
            dex_program: accs.dex_program.to_account_info(),
            token_program: accs.token_program.to_account_info(),
            rent: accs.rent.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

impl<'info> From<&SwapToUsdc<'info>> for CpiContext<'_, '_, '_, 'info, swap::Swap<'info>> {
    fn from(accs: &SwapToUsdc<'info>) -> Self {
        let program = accs.swap_program.to_account_info();
        let accounts = swap::Swap {
            market: swap::MarketAccounts {
                market: accs.market.market.clone(),
                open_orders: accs.market.open_orders.clone(),
                request_queue: accs.market.request_queue.clone(),
                event_queue: accs.market.event_queue.clone(),
                bids: accs.market.bids.clone(),
                asks: accs.market.asks.clone(),
                order_payer_token_account: accs.market.order_payer_token_account.clone(),
                coin_vault: accs.market.coin_vault.clone(),
                pc_vault: accs.market.pc_vault.clone(),
                vault_signer: accs.market.vault_signer.clone(),
                coin_wallet: accs.from_vault.to_account_info(),
            },
            authority: accs.officer.to_account_info(),
            pc_wallet: accs.usdc_vault.clone(),
            dex_program: accs.dex_program.to_account_info(),
            token_program: accs.token_program.to_account_info(),
            rent: accs.rent.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

impl<'info> From<&Distribute<'info>> for CpiContext<'_, '_, '_, 'info, token::Burn<'info>> {
    fn from(accs: &Distribute<'info>) -> Self {
        let program = accs.token_program.to_account_info();
        let accounts = token::Burn {
            mint: accs.mint.to_account_info(),
            to: accs.srm_vault.to_account_info(),
            authority: accs.officer.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

impl<'info> DropStakeReward<'info> {
    fn into_srm_reward(&self) -> CpiContext<'_, '_, '_, 'info, registry::DropReward<'info>> {
        let program = self.registry_program.clone();
        let accounts = registry::DropReward {
            registrar: ProgramAccount::try_from(&self.srm.registrar).unwrap(),
            reward_event_q: ProgramAccount::try_from(&self.srm.reward_event_q).unwrap(),
            pool_mint: self.srm.pool_mint.clone(),
            vendor: ProgramAccount::try_from(&self.srm.vendor).unwrap(),
            vendor_vault: CpiAccount::try_from(&self.srm.vendor_vault).unwrap(),
            depositor: self.stake.to_account_info(),
            depositor_authority: self.officer.to_account_info(),
            token_program: self.token_program.clone(),
            clock: self.clock.clone(),
            rent: self.rent.clone(),
        };
        CpiContext::new(program, accounts)
    }

    fn into_msrm_reward(&self) -> CpiContext<'_, '_, '_, 'info, registry::DropReward<'info>> {
        let program = self.registry_program.clone();
        let accounts = registry::DropReward {
            registrar: ProgramAccount::try_from(&self.msrm.registrar).unwrap(),
            reward_event_q: ProgramAccount::try_from(&self.msrm.reward_event_q).unwrap(),
            pool_mint: self.msrm.pool_mint.clone(),
            vendor: ProgramAccount::try_from(&self.msrm.vendor).unwrap(),
            vendor_vault: CpiAccount::try_from(&self.msrm.vendor_vault).unwrap(),
            depositor: self.stake.to_account_info(),
            depositor_authority: self.officer.to_account_info(),
            token_program: self.token_program.clone(),
            clock: self.clock.clone(),
            rent: self.rent.clone(),
        };
        CpiContext::new(program, accounts)
    }
}

impl<'info> Distribute<'info> {
    fn into_burn(&self) -> CpiContext<'_, '_, '_, 'info, token::Burn<'info>> {
        let program = self.token_program.clone();
        let accounts = token::Burn {
            mint: self.mint.clone(),
            to: self.srm_vault.to_account_info(),
            authority: self.officer.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }

    fn into_stake_transfer(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.clone();
        let accounts = token::Transfer {
            from: self.srm_vault.to_account_info(),
            to: self.stake.to_account_info(),
            authority: self.officer.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }

    fn into_treasury_transfer(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.clone();
        let accounts = token::Transfer {
            from: self.srm_vault.to_account_info(),
            to: self.treasury.to_account_info(),
            authority: self.officer.to_account_info(),
        };
        CpiContext::new(program, accounts)
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

#[error]
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
        return Err(ErrorCode::InvalidDistribution.into());
    }
    Ok(())
}

fn is_distribution_ready(accounts: &Distribute) -> Result<()> {
    if accounts.srm_vault.amount < 1_000_000 {
        return Err(ErrorCode::InsufficientDistributionAmount.into());
    }
    Ok(())
}

// `ixs` must be the Instructions sysvar.
fn is_not_trading(ixs: &AccountInfo) -> Result<()> {
    let data = ixs.try_borrow_data()?;
    match tx_instructions::load_instruction_at(1, &data) {
        Ok(_) => Err(ErrorCode::TooManyInstructions.into()),
        Err(_) => Ok(()),
    }
}

fn is_stake_reward_ready(accounts: &DropStakeReward) -> Result<()> {
    // Min drop is 15,0000 SRM.
    let min_reward: u64 = 15_000_000_000;
    if accounts.stake.amount < min_reward {
        return Err(ErrorCode::InsufficientStakeReward.into());
    }
    Ok(())
}

// Redefintions.
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
