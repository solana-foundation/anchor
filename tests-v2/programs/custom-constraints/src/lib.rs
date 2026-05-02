//! Third-party-style custom constraints exercising each of the four
//! `AccountConstraint` methods (`init`, `check`, `update`, `exit`).
//!
//! The `counter_ns` module defines four constraint markers, each
//! overriding exactly one method. Handlers drive each constraint via
//! the `#[account(...)]` syntax, and the integration tests in
//! `tests/custom_constraints.rs` assert the resulting on-chain state
//! to pin down WHERE in the lifecycle the derive routes each call.

extern crate alloc;

use anchor_lang_v2::prelude::*;

declare_id!("CC111111111111111111111111111111111111111111");

#[program]
pub mod custom_constraints {
    use super::*;

    /// `counter_ns::init_value = 5` on an `init` field.
    /// `InitValueConstraint::init` stamps `counter.value = 5` after
    /// BorshAccount creates + zero-fills the account.
    pub fn handle_init(_ctx: &mut Context<HandleInit>) -> Result<()> {
        Ok(())
    }

    /// `counter_ns::min_value = 10` on a non-init field.
    /// `MinValueConstraint::check` asserts `counter.value >= 10`.
    pub fn handle_check(_ctx: &mut Context<HandleCheck>) -> Result<()> {
        Ok(())
    }

    /// `update(counter_ns::set_value = 42)` on a `mut` field.
    /// `SetValueConstraint::update` writes `counter.value = 42`.
    pub fn handle_update(_ctx: &mut Context<HandleUpdate>) -> Result<()> {
        Ok(())
    }

    /// `counter_ns::bump_on_exit = 1` on a `mut` field.
    /// `BumpOnExitConstraint::exit` adds 1 to `counter.value` during
    /// `exit_accounts`. The integration test dispatches an otherwise
    /// no-op handler and checks the persisted value afterwards.
    pub fn handle_exit_bump(_ctx: &mut Context<HandleExitBump>) -> Result<()> {
        Ok(())
    }

    /// `init_if_needed` with `counter_ns::init_value = 5` —
    /// exercises that on the create branch, `init` fires, and on the
    /// exist branch, `check` fires (no-op for this constraint but the
    /// call is still emitted). Pairs with a separate non-init
    /// constraint `counter_ns::min_value` to prove the check path is
    /// reached.
    pub fn handle_init_if_needed(_ctx: &mut Context<HandleInitIfNeeded>) -> Result<()> {
        Ok(())
    }

    /// Boxed variant of `handle_init`, proving `AccountConstraint::init`
    /// forwards through `Box<T>` on an `init` field.
    pub fn handle_boxed_init(_ctx: &mut Context<HandleBoxedInit>) -> Result<()> {
        Ok(())
    }

    /// Boxed variant of `handle_exit_bump`, proving `AccountConstraint::exit`
    /// forwards through `Box<T>`.
    pub fn handle_boxed_exit_bump(_ctx: &mut Context<HandleBoxedExitBump>) -> Result<()> {
        Ok(())
    }

    /// Close a boxed counter to exercise `AnchorAccount::close` forwarding.
    pub fn handle_boxed_close(_ctx: &mut Context<HandleBoxedClose>) -> Result<()> {
        Ok(())
    }

    /// `TestAccount` declares `min_value` as required. This
    /// handler attaches it and compiles. The corresponding compile-fail
    /// case lives under `tests/ui_required_constraints/`.
    pub fn handle_required(_ctx: &mut Context<HandleRequired>) -> Result<()> {
        Ok(())
    }

    /// `TestMultiAccount` declares two requirements; both
    /// attached, declared in the opposite source order to prove
    /// the superset check is order-independent.
    pub fn handle_required_multi(_ctx: &mut Context<HandleRequiredMulti>) -> Result<()> {
        Ok(())
    }

    /// Required-constraint forwarding through `Box<TestAccount>`.
    pub fn handle_required_boxed(_ctx: &mut Context<HandleRequiredBoxed>) -> Result<()> {
        Ok(())
    }

    /// Spelling the required constraint as `update(...)` still satisfies
    /// the requirement — required-check is by marker-type-presence,
    /// not lifecycle phase.
    pub fn handle_required_via_update(
        _ctx: &mut Context<HandleRequiredViaUpdate>,
    ) -> Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Account data type — a plain Borsh-serialised counter.
// ---------------------------------------------------------------------------

#[account(borsh)]
pub struct Counter {
    pub value: u64,
}

// ---------------------------------------------------------------------------
// Custom constraint namespace. Each marker overrides exactly one of the
// four `AccountConstraint` methods to make routing observable.
// ---------------------------------------------------------------------------

pub mod counter_ns {
    use {
        super::Counter,
        anchor_lang_v2::{accounts::BorshAccount, AccountConstraint},
        solana_program_error::ProgramError,
    };

    /// `counter_ns::init_value = N` — stamps `counter.value = N`
    /// during the init phase (fires on `init` and on the create branch
    /// of `init_if_needed`).
    pub struct InitValueConstraint;

    impl AccountConstraint<BorshAccount<Counter>> for InitValueConstraint {
        type Value = u64;

        fn init(account: &mut BorshAccount<Counter>, value: &u64) -> Result<(), ProgramError> {
            account.value = *value;
            Ok(())
        }
    }

    /// `counter_ns::min_value = N` — asserts `counter.value >= N`.
    pub struct MinValueConstraint;

    impl AccountConstraint<BorshAccount<Counter>> for MinValueConstraint {
        type Value = u64;

        fn check(account: &BorshAccount<Counter>, min: &u64) -> Result<(), ProgramError> {
            if account.value < *min {
                return Err(ProgramError::InvalidAccountData);
            }
            Ok(())
        }
    }

    /// `counter_ns::set_value = N` — writes `counter.value = N`.
    /// Emitted only when paired with the `update(...)` wrapper.
    pub struct SetValueConstraint;

    impl AccountConstraint<BorshAccount<Counter>> for SetValueConstraint {
        type Value = u64;

        fn update(account: &mut BorshAccount<Counter>, value: &u64) -> Result<(), ProgramError> {
            account.value = *value;
            Ok(())
        }
    }

    /// `counter_ns::bump_on_exit = N` — adds `N` to `counter.value`
    /// during `exit_accounts`. The exit hook runs only on successful
    /// instructions, so a handler that errors must NOT see the bump.
    pub struct BumpOnExitConstraint;

    impl AccountConstraint<BorshAccount<Counter>> for BumpOnExitConstraint {
        type Value = u64;

        fn exit(account: &mut BorshAccount<Counter>, bump: &u64) -> Result<(), ProgramError> {
            account.value = account.value.saturating_add(*bump);
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Accounts structs
// ---------------------------------------------------------------------------

#[derive(Accounts)]
pub struct HandleInit {
    #[account(mut)]
    pub payer: Signer,
    #[account(
        init,
        payer = payer,
        space = 8 + 8, // disc + u64
        seeds = [b"counter"],
        bump,
        counter_ns::init_value = 5u64,
    )]
    pub counter: BorshAccount<Counter>,
    pub system_program: Program<System>,
}

#[derive(Accounts)]
pub struct HandleCheck {
    #[account(
        seeds = [b"counter"],
        bump,
        counter_ns::min_value = 10u64,
    )]
    pub counter: BorshAccount<Counter>,
}

#[derive(Accounts)]
pub struct HandleUpdate {
    #[account(
        mut,
        seeds = [b"counter"],
        bump,
        update(counter_ns::set_value = 42u64),
    )]
    pub counter: BorshAccount<Counter>,
}

#[derive(Accounts)]
pub struct HandleExitBump {
    #[account(
        mut,
        seeds = [b"counter"],
        bump,
        counter_ns::bump_on_exit = 1u64,
    )]
    pub counter: BorshAccount<Counter>,
}

#[derive(Accounts)]
pub struct HandleInitIfNeeded {
    #[account(mut)]
    pub payer: Signer,
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + 8,
        seeds = [b"counter"],
        bump,
        counter_ns::init_value = 5u64,
        counter_ns::min_value = 1u64,
    )]
    pub counter: BorshAccount<Counter>,
    pub system_program: Program<System>,
}

#[derive(Accounts)]
pub struct HandleBoxedInit {
    #[account(mut)]
    pub payer: Signer,
    #[account(
        init,
        payer = payer,
        space = 8 + 8,
        seeds = [b"boxed-counter"],
        bump,
        counter_ns::init_value = 9u64,
    )]
    pub counter: Box<BorshAccount<Counter>>,
    pub system_program: Program<System>,
}

#[derive(Accounts)]
pub struct HandleBoxedExitBump {
    #[account(
        mut,
        seeds = [b"boxed-counter"],
        bump,
        counter_ns::bump_on_exit = 2u64,
    )]
    pub counter: Box<BorshAccount<Counter>>,
}

#[derive(Accounts)]
pub struct HandleBoxedClose {
    #[account(mut, close = receiver, seeds = [b"boxed-counter"], bump)]
    pub counter: Box<BorshAccount<Counter>>,
    #[account(mut)]
    pub receiver: SystemAccount,
}

// ---------------------------------------------------------------------------
// `TestAccount` / `TestMultiAccount` — custom wrappers that declare required
// constraints.
//
// Thin newtypes around `BorshAccount<Counter>` whose `AnchorAccount` impls
// set `RequiredConstraints`. Every `#[account(...)]` usage must include the
// listed `counter_ns::*` constraints (`min_value` for `TestAccount`,
// `min_value` + `bump_on_exit` for `TestMultiAccount`) or the derive emits a
// compile error.
//
// Non-generic over `Counter` to sidestep restating `BorshAccount`'s wincode
// trait bounds at every impl site — these are test wrappers, not a public
// generic API.
// ---------------------------------------------------------------------------

pub struct TestAccount(BorshAccount<Counter>);

impl core::ops::Deref for TestAccount {
    type Target = Counter;
    fn deref(&self) -> &Counter {
        &*self.0
    }
}

impl core::ops::DerefMut for TestAccount {
    fn deref_mut(&mut self) -> &mut Counter {
        &mut *self.0
    }
}

impl AnchorAccount for TestAccount {
    type Data = Counter;
    type RequiredConstraints = required_constraints![counter_ns::MinValueConstraint];

    fn load(view: AccountView, program_id: &Address) -> core::result::Result<Self, ProgramError> {
        BorshAccount::<Counter>::load(view, program_id).map(Self)
    }

    unsafe fn load_mut(
        view: AccountView,
        program_id: &Address,
    ) -> core::result::Result<Self, ProgramError> {
        BorshAccount::<Counter>::load_mut(view, program_id).map(Self)
    }

    fn account(&self) -> &AccountView {
        self.0.account()
    }

    fn exit(&mut self) -> ProgramResult {
        self.0.exit()
    }

    fn close(&mut self, destination: AccountView) -> ProgramResult {
        self.0.close(destination)
    }
}

// `AccountInitialize` forward so `init` and `init_if_needed` work on
// `TestAccount`. Mirrors `Box<T>`'s pattern in lang-v2.
impl anchor_lang_v2::accounts::AccountInitialize for TestAccount {
    type Params<'a> =
        <BorshAccount<Counter> as anchor_lang_v2::accounts::AccountInitialize>::Params<'a>;

    fn create_and_initialize<'ix>(
        payer: &AccountView,
        target: &AccountView,
        space: usize,
        program_id: &Address,
        params: &Self::Params<'ix>,
        seeds: Option<&[&[u8]]>,
    ) -> core::result::Result<Self, ProgramError> {
        BorshAccount::<Counter>::create_and_initialize(
            payer, target, space, program_id, params, seeds,
        )
        .map(Self)
    }
}

// Per-constraint forwarders. The orphan rule disallows a blanket
// `impl<C> AccountConstraint<TestAccount> for C where C:
// AccountConstraint<BorshAccount<Counter>>` from a downstream crate
// (`C` is uncovered before the first local type), so we enumerate
// explicitly. Custom-account authors writing third-party constraints
// hit the same constraint and would write the impls similarly.
impl AccountConstraint<TestAccount> for counter_ns::MinValueConstraint {
    type Value = u64;
    fn check(account: &TestAccount, min: &u64) -> core::result::Result<(), ProgramError> {
        <Self as AccountConstraint<BorshAccount<Counter>>>::check(&account.0, min)
    }
    fn update(account: &mut TestAccount, min: &u64) -> core::result::Result<(), ProgramError> {
        <Self as AccountConstraint<BorshAccount<Counter>>>::update(&mut account.0, min)
    }
}

// A second wrapper that requires TWO constraints, used to test multi-
// element required lists and order-independence of the superset check.
pub struct TestMultiAccount(BorshAccount<Counter>);

impl core::ops::Deref for TestMultiAccount {
    type Target = Counter;
    fn deref(&self) -> &Counter {
        &*self.0
    }
}

impl core::ops::DerefMut for TestMultiAccount {
    fn deref_mut(&mut self) -> &mut Counter {
        &mut *self.0
    }
}

impl AnchorAccount for TestMultiAccount {
    type Data = Counter;
    type RequiredConstraints = required_constraints![
        counter_ns::MinValueConstraint,
        counter_ns::BumpOnExitConstraint,
    ];

    fn load(view: AccountView, program_id: &Address) -> core::result::Result<Self, ProgramError> {
        BorshAccount::<Counter>::load(view, program_id).map(Self)
    }

    unsafe fn load_mut(
        view: AccountView,
        program_id: &Address,
    ) -> core::result::Result<Self, ProgramError> {
        BorshAccount::<Counter>::load_mut(view, program_id).map(Self)
    }

    fn account(&self) -> &AccountView {
        self.0.account()
    }

    fn exit(&mut self) -> ProgramResult {
        self.0.exit()
    }
}

impl AccountConstraint<TestMultiAccount> for counter_ns::MinValueConstraint {
    type Value = u64;
    fn check(account: &TestMultiAccount, min: &u64) -> core::result::Result<(), ProgramError> {
        <Self as AccountConstraint<BorshAccount<Counter>>>::check(&account.0, min)
    }
}

impl AccountConstraint<TestMultiAccount> for counter_ns::BumpOnExitConstraint {
    type Value = u64;
    fn exit(account: &mut TestMultiAccount, bump: &u64) -> core::result::Result<(), ProgramError> {
        <Self as AccountConstraint<BorshAccount<Counter>>>::exit(&mut account.0, bump)
    }
}

// ---------------------------------------------------------------------------
// Accounts structs that exercise required-constraint enforcement.
// ---------------------------------------------------------------------------

#[derive(Accounts)]
pub struct HandleRequired {
    #[account(
        seeds = [b"counter"],
        bump,
        counter_ns::min_value = 0u64,
    )]
    pub counter: TestAccount,
}

#[derive(Accounts)]
pub struct HandleRequiredMulti {
    // Required = [MinValueConstraint, BumpOnExitConstraint]
    // Attached in reverse source order — superset check ignores order.
    #[account(
        mut,
        seeds = [b"counter"],
        bump,
        counter_ns::bump_on_exit = 0u64,
        counter_ns::min_value = 0u64,
    )]
    pub counter: TestMultiAccount,
}

#[derive(Accounts)]
pub struct HandleRequiredBoxed {
    // RequiredConstraints forwards through Box<T> via the lang-v2 impl.
    #[account(
        seeds = [b"counter"],
        bump,
        counter_ns::min_value = 0u64,
    )]
    pub counter: Box<TestAccount>,
}

#[derive(Accounts)]
pub struct HandleRequiredViaUpdate {
    // Required marker is `MinValueConstraint`; spelled inside `update(...)`.
    // The required-check is by marker-type presence — the wrapper choice
    // doesn't matter for fulfillment.
    #[account(
        mut,
        seeds = [b"counter"],
        bump,
        update(counter_ns::min_value = 0u64),
    )]
    pub counter: TestAccount,
}
