//! An IDO pool program implementing the Mango Markets token sale design here:
//! https://docs.mango.markets/litepaper#token-sale.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::token::{self, Burn, Mint, MintTo, TokenAccount, Transfer};

#[program]
pub mod ido_pool {
    use super::*;

    #[access_control(InitializePool::accounts(&ctx, nonce) future_start_time(&ctx, start_ido_ts))]
    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        num_ido_tokens: u64,
        nonce: u8,
        start_ido_ts: i64,
        end_deposits_ts: i64,
        end_ido_ts: i64,
    ) -> Result<()> {
        if !(start_ido_ts < end_deposits_ts && end_deposits_ts < end_ido_ts) {
            return Err(ErrorCode::SeqTimes.into());
        }

        let pool_account = &mut ctx.accounts.pool_account;
        pool_account.redeemable_mint = *ctx.accounts.redeemable_mint.to_account_info().key;
        pool_account.pool_watermelon = *ctx.accounts.pool_watermelon.to_account_info().key;
        pool_account.watermelon_mint = ctx.accounts.pool_watermelon.mint;
        pool_account.pool_usdc = *ctx.accounts.pool_usdc.to_account_info().key;
        pool_account.distribution_authority = *ctx.accounts.distribution_authority.key;
        pool_account.nonce = nonce;
        pool_account.num_ido_tokens = num_ido_tokens;
        pool_account.start_ido_ts = start_ido_ts;
        pool_account.end_deposits_ts = end_deposits_ts;
        pool_account.end_ido_ts = end_ido_ts;

        // Transfer Watermelon from creator to pool account.
        let cpi_accounts = Transfer {
            from: ctx.accounts.creator_watermelon.to_account_info(),
            to: ctx.accounts.pool_watermelon.to_account_info(),
            authority: ctx.accounts.distribution_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, num_ido_tokens)?;

        Ok(())
    }

    #[access_control(unrestricted_phase(&ctx))]
    pub fn exchange_usdc_for_redeemable(
        ctx: Context<ExchangeUsdcForRedeemable>,
        amount: u64,
    ) -> Result<()> {
        // While token::transfer will check this, we prefer a verbose err msg.
        if ctx.accounts.user_usdc.amount < amount {
            return Err(ErrorCode::LowUsdc.into());
        }

        // Transfer user's USDC to pool USDC account.
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_usdc.to_account_info(),
            to: ctx.accounts.pool_usdc.to_account_info(),
            authority: ctx.accounts.user_authority.clone(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        // Mint Redeemable to user Redeemable account.
        let seeds = &[
            ctx.accounts.pool_account.watermelon_mint.as_ref(),
            &[ctx.accounts.pool_account.nonce],
        ];
        let signer = &[&seeds[..]];
        let cpi_accounts = MintTo {
            mint: ctx.accounts.redeemable_mint.to_account_info(),
            to: ctx.accounts.user_redeemable.to_account_info(),
            authority: ctx.accounts.pool_signer.clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::mint_to(cpi_ctx, amount)?;

        Ok(())
    }

    #[access_control(withdraw_only_phase(&ctx))]
    pub fn exchange_redeemable_for_usdc(
        ctx: Context<ExchangeRedeemableForUsdc>,
        amount: u64,
    ) -> Result<()> {
        // While token::burn will check this, we prefer a verbose err msg.
        if ctx.accounts.user_redeemable.amount < amount {
            return Err(ErrorCode::LowRedeemable.into());
        }

        // Burn the user's redeemable tokens.
        let cpi_accounts = Burn {
            mint: ctx.accounts.redeemable_mint.to_account_info(),
            to: ctx.accounts.user_redeemable.to_account_info(),
            authority: ctx.accounts.user_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::burn(cpi_ctx, amount)?;

        // Transfer USDC from pool account to user.
        let seeds = &[
            ctx.accounts.pool_account.watermelon_mint.as_ref(),
            &[ctx.accounts.pool_account.nonce],
        ];
        let signer = &[&seeds[..]];
        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_usdc.to_account_info(),
            to: ctx.accounts.user_usdc.to_account_info(),
            authority: ctx.accounts.pool_signer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, amount)?;

        Ok(())
    }

    #[access_control(ido_over(&ctx.accounts.pool_account, &ctx.accounts.clock))]
    pub fn exchange_redeemable_for_watermelon(
        ctx: Context<ExchangeRedeemableForWatermelon>,
        amount: u64,
    ) -> Result<()> {
        // While token::burn will check this, we prefer a verbose err msg.
        if ctx.accounts.user_redeemable.amount < amount {
            return Err(ErrorCode::LowRedeemable.into());
        }

        // Calculate watermelon tokens due.
        let watermelon_amount = (amount as u128)
            .checked_mul(ctx.accounts.pool_watermelon.amount as u128)
            .unwrap()
            .checked_div(ctx.accounts.redeemable_mint.supply as u128)
            .unwrap();

        // Burn the user's redeemable tokens.
        let cpi_accounts = Burn {
            mint: ctx.accounts.redeemable_mint.to_account_info(),
            to: ctx.accounts.user_redeemable.to_account_info(),
            authority: ctx.accounts.user_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::burn(cpi_ctx, amount)?;

        // Transfer Watermelon from pool account to user.
        let seeds = &[
            ctx.accounts.pool_account.watermelon_mint.as_ref(),
            &[ctx.accounts.pool_account.nonce],
        ];
        let signer = &[&seeds[..]];
        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_watermelon.to_account_info(),
            to: ctx.accounts.user_watermelon.to_account_info(),
            authority: ctx.accounts.pool_signer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, watermelon_amount as u64)?;

        Ok(())
    }

    #[access_control(ido_over(&ctx.accounts.pool_account, &ctx.accounts.clock))]
    pub fn withdraw_pool_usdc(ctx: Context<WithdrawPoolUsdc>) -> Result<()> {
        // Transfer total USDC from pool account to creator account.
        let seeds = &[
            ctx.accounts.pool_account.watermelon_mint.as_ref(),
            &[ctx.accounts.pool_account.nonce],
        ];
        let signer = &[&seeds[..]];
        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_usdc.to_account_info(),
            to: ctx.accounts.creator_usdc.to_account_info(),
            authority: ctx.accounts.pool_signer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, ctx.accounts.pool_usdc.amount)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(init)]
    pub pool_account: ProgramAccount<'info, PoolAccount>,
    pub pool_signer: AccountInfo<'info>,
    #[account(
        constraint = redeemable_mint.mint_authority == COption::Some(*pool_signer.key),
        constraint = redeemable_mint.supply == 0
    )]
    pub redeemable_mint: CpiAccount<'info, Mint>,
    #[account(constraint = usdc_mint.decimals == redeemable_mint.decimals)]
    pub usdc_mint: CpiAccount<'info, Mint>,
    #[account(mut, constraint = pool_watermelon.owner == *pool_signer.key)]
    pub pool_watermelon: CpiAccount<'info, TokenAccount>,
    #[account(constraint = pool_usdc.owner == *pool_signer.key)]
    pub pool_usdc: CpiAccount<'info, TokenAccount>,
    #[account(signer)]
    pub distribution_authority: AccountInfo<'info>,
    #[account(mut, constraint = creator_watermelon.owner == *distribution_authority.key)]
    pub creator_watermelon: CpiAccount<'info, TokenAccount>,
    #[account(constraint = token_program.key == &token::ID)]
    pub token_program: AccountInfo<'info>,
    pub clock: Sysvar<'info, Clock>,
}

impl<'info> InitializePool<'info> {
    fn accounts(ctx: &Context<InitializePool<'info>>, nonce: u8) -> Result<()> {
        let expected_signer = Pubkey::create_program_address(
            &[ctx.accounts.pool_watermelon.mint.as_ref(), &[nonce]],
            ctx.program_id,
        )
        .map_err(|_| ErrorCode::InvalidNonce)?;
        if ctx.accounts.pool_signer.key != &expected_signer {
            return Err(ErrorCode::InvalidNonce.into());
        }
        Ok(())
    }
}

#[derive(Accounts)]
pub struct ExchangeUsdcForRedeemable<'info> {
    #[account(has_one = redeemable_mint, has_one = pool_usdc)]
    pub pool_account: ProgramAccount<'info, PoolAccount>,
    #[account(seeds = [pool_account.watermelon_mint.as_ref(), &[pool_account.nonce]])]
    pool_signer: AccountInfo<'info>,
    #[account(
        mut,
        constraint = redeemable_mint.mint_authority == COption::Some(*pool_signer.key)
    )]
    pub redeemable_mint: CpiAccount<'info, Mint>,
    #[account(mut, constraint = pool_usdc.owner == *pool_signer.key)]
    pub pool_usdc: CpiAccount<'info, TokenAccount>,
    #[account(signer)]
    pub user_authority: AccountInfo<'info>,
    #[account(mut, constraint = user_usdc.owner == *user_authority.key)]
    pub user_usdc: CpiAccount<'info, TokenAccount>,
    #[account(mut, constraint = user_redeemable.owner == *user_authority.key)]
    pub user_redeemable: CpiAccount<'info, TokenAccount>,
    #[account(constraint = token_program.key == &token::ID)]
    pub token_program: AccountInfo<'info>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct ExchangeRedeemableForUsdc<'info> {
    #[account(has_one = redeemable_mint, has_one = pool_usdc)]
    pub pool_account: ProgramAccount<'info, PoolAccount>,
    #[account(seeds = [pool_account.watermelon_mint.as_ref(), &[pool_account.nonce]])]
    pool_signer: AccountInfo<'info>,
    #[account(
        mut,
        constraint = redeemable_mint.mint_authority == COption::Some(*pool_signer.key)
    )]
    pub redeemable_mint: CpiAccount<'info, Mint>,
    #[account(mut, constraint = pool_usdc.owner == *pool_signer.key)]
    pub pool_usdc: CpiAccount<'info, TokenAccount>,
    #[account(signer)]
    pub user_authority: AccountInfo<'info>,
    #[account(mut, constraint = user_usdc.owner == *user_authority.key)]
    pub user_usdc: CpiAccount<'info, TokenAccount>,
    #[account(mut, constraint = user_redeemable.owner == *user_authority.key)]
    pub user_redeemable: CpiAccount<'info, TokenAccount>,
    #[account(constraint = token_program.key == &token::ID)]
    pub token_program: AccountInfo<'info>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct ExchangeRedeemableForWatermelon<'info> {
    #[account(has_one = redeemable_mint, has_one = pool_watermelon)]
    pub pool_account: ProgramAccount<'info, PoolAccount>,
    #[account(seeds = [pool_account.watermelon_mint.as_ref(), &[pool_account.nonce]])]
    pool_signer: AccountInfo<'info>,
    #[account(
        mut,
        constraint = redeemable_mint.mint_authority == COption::Some(*pool_signer.key)
    )]
    pub redeemable_mint: CpiAccount<'info, Mint>,
    #[account(mut, constraint = pool_watermelon.owner == *pool_signer.key)]
    pub pool_watermelon: CpiAccount<'info, TokenAccount>,
    #[account(signer)]
    pub user_authority: AccountInfo<'info>,
    #[account(mut, constraint = user_watermelon.owner == *user_authority.key)]
    pub user_watermelon: CpiAccount<'info, TokenAccount>,
    #[account(mut, constraint = user_redeemable.owner == *user_authority.key)]
    pub user_redeemable: CpiAccount<'info, TokenAccount>,
    #[account(constraint = token_program.key == &token::ID)]
    pub token_program: AccountInfo<'info>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct WithdrawPoolUsdc<'info> {
    #[account(has_one = pool_usdc, has_one = distribution_authority)]
    pub pool_account: ProgramAccount<'info, PoolAccount>,
    #[account(seeds = [pool_account.watermelon_mint.as_ref(), &[pool_account.nonce]])]
    pub pool_signer: AccountInfo<'info>,
    #[account(mut, constraint = pool_usdc.owner == *pool_signer.key)]
    pub pool_usdc: CpiAccount<'info, TokenAccount>,
    #[account(signer)]
    pub distribution_authority: AccountInfo<'info>,
    #[account(mut, constraint = creator_usdc.owner == *distribution_authority.key)]
    pub creator_usdc: CpiAccount<'info, TokenAccount>,
    #[account(constraint = token_program.key == &token::ID)]
    pub token_program: AccountInfo<'info>,
    pub clock: Sysvar<'info, Clock>,
}

#[account]
pub struct PoolAccount {
    pub redeemable_mint: Pubkey,
    pub pool_watermelon: Pubkey,
    pub watermelon_mint: Pubkey,
    pub pool_usdc: Pubkey,
    pub distribution_authority: Pubkey,
    pub nonce: u8,
    pub num_ido_tokens: u64,
    pub start_ido_ts: i64,
    pub end_deposits_ts: i64,
    pub end_ido_ts: i64,
}

#[error]
pub enum ErrorCode {
    #[msg("IDO must start in the future")]
    IdoFuture,
    #[msg("IDO times are non-sequential")]
    SeqTimes,
    #[msg("IDO has not started")]
    StartIdoTime,
    #[msg("Deposits period has ended")]
    EndDepositsTime,
    #[msg("IDO has ended")]
    EndIdoTime,
    #[msg("IDO has not finished yet")]
    IdoNotOver,
    #[msg("Insufficient USDC")]
    LowUsdc,
    #[msg("Insufficient redeemable tokens")]
    LowRedeemable,
    #[msg("USDC total and redeemable total don't match")]
    UsdcNotEqRedeem,
    #[msg("Given nonce is invalid")]
    InvalidNonce,
}

// Access control modifiers.

// Asserts the IDO starts in the future.
fn future_start_time<'info>(ctx: &Context<InitializePool<'info>>, start_ido_ts: i64) -> Result<()> {
    if !(ctx.accounts.clock.unix_timestamp < start_ido_ts) {
        return Err(ErrorCode::IdoFuture.into());
    }
    Ok(())
}

// Asserts the IDO is in the first phase.
fn unrestricted_phase<'info>(ctx: &Context<ExchangeUsdcForRedeemable<'info>>) -> Result<()> {
    if !(ctx.accounts.pool_account.start_ido_ts < ctx.accounts.clock.unix_timestamp) {
        return Err(ErrorCode::StartIdoTime.into());
    } else if !(ctx.accounts.clock.unix_timestamp < ctx.accounts.pool_account.end_deposits_ts) {
        return Err(ErrorCode::EndDepositsTime.into());
    }
    Ok(())
}

// Asserts the IDO is in the second phase.
fn withdraw_only_phase(ctx: &Context<ExchangeRedeemableForUsdc>) -> Result<()> {
    if !(ctx.accounts.pool_account.start_ido_ts < ctx.accounts.clock.unix_timestamp) {
        return Err(ErrorCode::StartIdoTime.into());
    } else if !(ctx.accounts.clock.unix_timestamp < ctx.accounts.pool_account.end_ido_ts) {
        return Err(ErrorCode::EndIdoTime.into());
    }
    Ok(())
}

// Asserts the IDO sale period has ended, based on the current timestamp.
fn ido_over<'info>(
    pool_account: &ProgramAccount<'info, PoolAccount>,
    clock: &Sysvar<'info, Clock>,
) -> Result<()> {
    if !(pool_account.end_ido_ts < clock.unix_timestamp) {
        return Err(ErrorCode::IdoNotOver.into());
    }
    Ok(())
}
