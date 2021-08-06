// Note. This example depends on unreleased Serum DEX changes.

use anchor_lang::prelude::*;
use anchor_spl::dex::serum_dex::instruction::{CancelOrderInstructionV2, NewOrderInstructionV3};
use anchor_spl::dex::{
    Context, Logger, MarketMiddleware, MarketProxy, OpenOrdersPda, ReferralFees,
};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::rent;

/// # Permissioned Markets
///
/// This demonstrates how to create "permissioned markets" on Serum via a proxy.
/// A permissioned market is a regular Serum market with an additional
/// open orders authority, which must sign every transaction to create or
/// close an open orders account.
///
/// In practice, what this means is that one can create a program that acts
/// as this authority *and* that marks its own PDAs as the *owner* of all
/// created open orders accounts, making the program the sole arbiter over
/// who can trade on a given market.
///
/// For example, this example forces all trades that execute on this market
/// to set the referral to a hardcoded address--`referral::ID`--and requires
/// the client to pass in an identity token, authorizing the user.
///
/// # Extending the proxy via middleware
///
/// To implement a custom proxy, one can implement the `MarketMiddleware` trait
/// to intercept, modify, and perform any access control on DEX requests before
/// they get forwarded to the orderbook. These middleware can be mixed and
/// matched. Note, however, that the order of middleware matters since they can
/// mutate the request.
///
/// One useful pattern is to treat the request like layers of an onion, where
/// each middleware unwraps the request by stripping accounts and instruction
/// data before relaying it to the next middleware and ultimately to the
/// orderbook. This allows one to easily extend the behavior of a proxy by
/// adding a custom middleware that may process information that is unknown to
/// any other middleware or to the DEX.
///
/// After adding a middleware, the only additional requirement, of course, is
/// to make sure the client sending transactions does the same, but in reverse.
/// It should wrap the transaction in the opposite order. For convenience, an
/// identical abstraction is provided in the JavaScript client.
///
/// # Alternatives to middleware
///
/// Note that this middleware abstraction is not required to host a
/// permissioned market. One could write a regular program that manages the PDAs
/// and CPI invocations oneself, if desired.
#[program]
pub mod permissioned_markets_middleware {
    use super::*;
    pub fn entry(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
        MarketProxy::new()
            .middleware(&mut Logger)
            .middleware(&mut Identity)
            .middleware(&mut ReferralFees::new(referral::ID))
            .middleware(&mut OpenOrdersPda::new())
            .run(program_id, accounts, data)
    }
}

/// Performs token based authorization, confirming the identity of the user.
/// The identity token must be given as the fist account.
struct Identity;

impl MarketMiddleware for Identity {
    /// Accounts:
    ///
    /// 0. Authorization token.
    /// ..
    fn init_open_orders(&self, ctx: &mut Context) -> ProgramResult {
        verify_and_strip_auth(ctx)
    }

    /// Accounts:
    ///
    /// 0. Authorization token.
    /// ..
    fn new_order_v3(&self, ctx: &mut Context, _ix: &NewOrderInstructionV3) -> ProgramResult {
        verify_and_strip_auth(ctx)
    }

    /// Accounts:
    ///
    /// 0. Authorization token.
    /// ..
    fn cancel_order_v2(&self, ctx: &mut Context, _ix: &CancelOrderInstructionV2) -> ProgramResult {
        verify_and_strip_auth(ctx)
    }

    /// Accounts:
    ///
    /// 0. Authorization token.
    /// ..
    fn cancel_order_by_client_id_v2(&self, ctx: &mut Context, _client_id: u64) -> ProgramResult {
        verify_and_strip_auth(ctx)
    }

    /// Accounts:
    ///
    /// 0. Authorization token.
    /// ..
    fn settle_funds(&self, ctx: &mut Context) -> ProgramResult {
        verify_and_strip_auth(ctx)
    }

    /// Accounts:
    ///
    /// 0. Authorization token.
    /// ..
    fn close_open_orders(&self, ctx: &mut Context) -> ProgramResult {
        verify_and_strip_auth(ctx)
    }

    /// Accounts:
    ///
    /// 0. Authorization token.
    /// ..
    fn fallback(&self, ctx: &mut Context) -> ProgramResult {
        verify_and_strip_auth(ctx)
    }
}

// Utils.

fn verify_and_strip_auth(ctx: &mut Context) -> ProgramResult {
    // The rent sysvar is used as a dummy example of an identity token.
    let auth = &ctx.accounts[0];
    require!(auth.key == &rent::ID, InvalidAuth);

    // Strip off the account before possing on the message.
    ctx.accounts = (&ctx.accounts[1..]).to_vec();

    Ok(())
}

// Error.

#[error]
pub enum ErrorCode {
    #[msg("Invalid auth token provided")]
    InvalidAuth,
}

// Constants.

pub mod referral {
    // This is a dummy address for testing. Do not use in production.
    solana_program::declare_id!("3oSfkjQZKCneYvsCTZc9HViGAPqR8pYr4h9YeGB5ZxHf");
}
