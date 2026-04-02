use {
    anchor_lang::{
        context::CpiContext,
        pinocchio_runtime::{account_view::AccountView, pubkey::Pubkey},
        Accounts, Result,
    },
    borsh::BorshDeserialize,
    solana_stake_interface::{
        self as stake,
        program::ID,
        state::{StakeAuthorize, StakeStateV2},
    },
    std::ops::Deref,
};

// CPI functions

pub fn authorize<'info>(
    ctx: CpiContext<'_, '_, Authorize<'info>>,
    stake_authorize: StakeAuthorize,
    custodian: Option<AccountView>,
) -> Result<()> {
    let ix = stake::instruction::authorize(
        ctx.accounts.stake.address(),
        ctx.accounts.authorized.address(),
        ctx.accounts.new_authorized.address(),
        stake_authorize,
        custodian.as_ref().map(|c| c.address()),
    );
    let mut account_infos = vec![
        ctx.accounts.stake,
        ctx.accounts.clock,
        ctx.accounts.authorized,
    ];
    if let Some(c) = custodian {
        account_infos.push(c);
    }
    anchor_lang::pinocchio_runtime::program::invoke_signed(&ix, &account_infos, ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn withdraw<'info>(
    ctx: CpiContext<'_, '_, Withdraw<'info>>,
    amount: u64,
    custodian: Option<AccountView>,
) -> Result<()> {
    let ix = stake::instruction::withdraw(
        ctx.accounts.stake.address(),
        ctx.accounts.withdrawer.address(),
        ctx.accounts.to.address(),
        amount,
        custodian.as_ref().map(|c| c.address()),
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
    anchor_lang::pinocchio_runtime::program::invoke_signed(&ix, &account_infos, ctx.signer_seeds)
        .map_err(Into::into)
}

pub fn deactivate_stake<'info>(ctx: CpiContext<'_, '_, DeactivateStake<'info>>) -> Result<()> {
    let ix = stake::instruction::deactivate_stake(
        ctx.accounts.stake.address(),
        ctx.accounts.staker.address(),
    );
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[ctx.accounts.stake, ctx.accounts.clock, ctx.accounts.staker],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

// CPI accounts

#[derive(Accounts)]
pub struct Authorize<'info> {
    /// The stake account to be updated
    pub stake: AccountView,

    /// The existing authority
    pub authorized: AccountView,

    /// The new authority to replace the existing authority
    pub new_authorized: AccountView,

    /// Clock sysvar
    pub clock: AccountView,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    /// The stake account to be updated
    pub stake: AccountView,

    /// The stake account's withdraw authority
    pub withdrawer: AccountView,

    /// Account to send withdrawn lamports to
    pub to: AccountView,

    /// Clock sysvar
    pub clock: AccountView,

    /// StakeHistory sysvar
    pub stake_history: AccountView,
}

#[derive(Accounts)]
pub struct DeactivateStake<'info> {
    /// The stake account to be deactivated
    pub stake: AccountView,

    /// The stake account's stake authority
    pub staker: AccountView,

    /// Clock sysvar
    pub clock: AccountView,
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
