use anchor_lang::{
    context::CpiContext,
    solana_program::{account_info::AccountInfo, pubkey::Pubkey},
    Accounts, Result,
};
use borsh::BorshDeserialize;
use solana_stake_interface::{
    self as stake,
    program::ID,
    state::{StakeAuthorize, StakeStateV2},
};
use std::ops::Deref;

// CPI functions

pub fn authorize(
    ctx: CpiContext<'_, '_, Authorize>,
    stake_authorize: StakeAuthorize,
    custodian: Option<AccountInfo>,
) -> Result<()> {
    let ix = stake::instruction::authorize(
        ctx.accounts.stake.key,
        ctx.accounts.authorized.key,
        ctx.accounts.new_authorized.key,
        stake_authorize,
        custodian.as_ref().map(|c| c.key),
    );
    let mut account_infos = vec![
        ctx.accounts.stake,
        ctx.accounts.clock,
        ctx.accounts.authorized,
    ];
    if let Some(c) = custodian {
        account_infos.push(c);
    }
    anchor_lang::solana_program::program::invoke_signed(&ix, &account_infos, ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn withdraw(
    ctx: CpiContext<'_, '_, Withdraw>,
    amount: u64,
    custodian: Option<AccountInfo>,
) -> Result<()> {
    let ix = stake::instruction::withdraw(
        ctx.accounts.stake.key,
        ctx.accounts.withdrawer.key,
        ctx.accounts.to.key,
        amount,
        custodian.as_ref().map(|c| c.key),
    );
    let mut account_infos = vec![
        ctx.accounts.stake,
        ctx.accounts.to,
        ctx.accounts.clock,
        ctx.accounts.stake_history,
        ctx.accounts.withdrawer,
    ];
    if let Some(c) = custodian {
        account_infos.push(c);
    }
    anchor_lang::solana_program::program::invoke_signed(&ix, &account_infos, ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn deactivate_stake(
    ctx: CpiContext<'_, '_, DeactivateStake>,
) -> Result<()> {
    let ix = stake::instruction::deactivate_stake(ctx.accounts.stake.key, ctx.accounts.staker.key);
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.stake, ctx.accounts.clock, ctx.accounts.staker],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

// CPI accounts

#[derive(Accounts)]
pub struct Authorize {
    /// The stake account to be updated
    pub stake: AccountInfo,

    /// The existing authority
    pub authorized: AccountInfo,

    /// The new authority to replace the existing authority
    pub new_authorized: AccountInfo,

    /// Clock sysvar
    pub clock: AccountInfo,
}

#[derive(Accounts)]
pub struct Withdraw {
    /// The stake account to be updated
    pub stake: AccountInfo,

    /// The stake account's withdraw authority
    pub withdrawer: AccountInfo,

    /// Account to send withdrawn lamports to
    pub to: AccountInfo,

    /// Clock sysvar
    pub clock: AccountInfo,

    /// StakeHistory sysvar
    pub stake_history: AccountInfo,
}

#[derive(Accounts)]
pub struct DeactivateStake {
    /// The stake account to be deactivated
    pub stake: AccountInfo,

    /// The stake account's stake authority
    pub staker: AccountInfo,

    /// Clock sysvar
    pub clock: AccountInfo,
}

// State

#[derive(Clone)]
pub struct StakeAccount(StakeStateV2);

impl anchor_lang::AccountDeserialize for StakeAccount {
    fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        Self::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        StakeStateV2::deserialize(buf).map(Self).map_err(Into::into)
    }
}

impl anchor_lang::AccountSerialize for StakeAccount {}

impl anchor_lang::Owner for StakeAccount {
    fn owner() -> Pubkey {
        ID
    }
}

impl Deref for StakeAccount {
    type Target = StakeStateV2;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct Stake;

impl anchor_lang::Id for Stake {
    fn id() -> Pubkey {
        ID
    }
}
