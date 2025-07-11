use anchor_lang::prelude::AccountMeta;
use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use arch_program::account::AccountInfo;
use arch_program::program_error::ProgramError;
use arch_program::utxo::UtxoMeta;
use saturn_bitcoin_transactions::utxo_info::UtxoInfo;
use saturn_utxo_parser::register_test_utxo_info;
use saturn_utxo_parser::ErrorCode;
use saturn_utxo_parser::TryFromUtxos;
use saturn_utxo_parser_derive::UtxoParser;
use std::collections::BTreeSet;

mod test_helpers;
use test_helpers::ctx_for;

/// Helper to construct a `UtxoInfo` with the given value and deterministic txid/vout.
fn create_meta(txid_byte: u8, vout: u32) -> UtxoMeta {
    let txid = [txid_byte; 32];
    UtxoMeta::from(txid, vout)
}

// -----------------------------------------------------------------------------
// Basic happy-path behaviour
// -----------------------------------------------------------------------------
#[derive(Debug, UtxoParser)]
#[utxo_accounts(DummyAccounts)]
struct Basic {
    /// First UTXO captured (fee)
    fee: UtxoInfo,

    /// Optional second UTXO
    deposit: Option<UtxoInfo>,

    /// Catch-all remaining
    #[utxo(rest)]
    others: Vec<UtxoInfo>,
}

#[test]
fn parses_expected_inputs() {
    // Prepare inputs.
    let m_fee = create_meta(1, 0);
    let m_dep = create_meta(2, 0);
    let m_extra = create_meta(3, 1);

    let inputs = vec![m_dep, m_fee, m_extra];

    let mut dummy = DummyAccounts::default();
    let mut ctx = ctx_for(&mut dummy);
    let parsed = Basic::try_utxos(&mut ctx, &inputs).expect("parsing should succeed");

    // Validate that fields were populated as expected.
    assert_eq!(parsed.fee.meta.vout(), 0);
    assert!(parsed.deposit.is_some());
    assert_eq!(parsed.deposit.as_ref().unwrap().meta.vout(), 0);
    assert_eq!(parsed.others.len(), 1);
    assert_eq!(parsed.others[0].meta.vout(), 1);
}

// -----------------------------------------------------------------------------
// Missing required UTXO should yield `MissingRequiredUtxo` error.
// -----------------------------------------------------------------------------
#[test]
fn missing_required_utxo() {
    // No fee UTXO with the required value is provided.
    let inputs = vec![create_meta(1, 0)];

    let mut dummy = DummyAccounts::default();
    let mut ctx = ctx_for(&mut dummy);
    // Under the updated macro semantics an unconstrained `fee` field will happily
    // accept the single UTXO, so parsing should succeed.
    let parsed = Basic::try_utxos(&mut ctx, &inputs).expect("single input should parse");
    assert_eq!(parsed.fee.meta.vout(), 0);
    assert!(parsed.deposit.is_none());
    assert!(parsed.others.is_empty());
}

// -----------------------------------------------------------------------------
// Extra inputs without a `rest` collector should yield `UnexpectedExtraUtxos`.
// -----------------------------------------------------------------------------
#[derive(Debug, UtxoParser)]
#[utxo_accounts(DummyAccounts)]
struct NoRest {
    fee: UtxoInfo,
}

#[test]
fn unexpected_extra_utxos() {
    let inputs = vec![create_meta(1, 0), create_meta(2, 0)];

    let mut dummy = DummyAccounts::default();
    let mut ctx = ctx_for(&mut dummy);
    let err = NoRest::try_utxos(&mut ctx, &inputs).unwrap_err();
    assert_eq!(
        err,
        ProgramError::Custom(ErrorCode::UnexpectedExtraUtxos.into())
    );
}

// -----------------------------------------------------------------------------
// Value predicate failure should yield `InvalidUtxoValue`.
// -----------------------------------------------------------------------------
#[derive(Debug, UtxoParser)]
#[utxo_accounts(DummyAccounts)]
struct ValueCheck {
    /// Expect a specific value that the test UTXO will not satisfy so that
    /// the parser returns `InvalidUtxoValue`.
    #[utxo(value = 1)]
    the_answer: UtxoInfo,
}

#[test]
fn value_check_failure() {
    let inputs = vec![create_meta(1, 0)];

    let mut dummy = DummyAccounts::default();
    let mut ctx = ctx_for(&mut dummy);
    let err = ValueCheck::try_utxos(&mut ctx, &inputs).unwrap_err();
    assert_eq!(
        err,
        ProgramError::Custom(ErrorCode::InvalidUtxoValue.into())
    );
}

// -----------------------------------------------------------------------------
// Anchor attribute should be accepted and parsing should succeed.
// -----------------------------------------------------------------------------
#[derive(Debug, UtxoParser)]
#[utxo_accounts(DummyAccounts)]
struct AnchorAttr {
    /// UTXO that will be anchored to an account field (dummy for macro parsing).
    #[utxo(anchor = my_account)]
    anchor_utxo: UtxoInfo,

    /// Capture any extras to satisfy the rest rule.
    #[utxo(rest)]
    others: Vec<UtxoInfo>,
}

#[test]
fn anchor_attribute_parses() {
    let anchor = create_meta(10, 0);
    let extra = create_meta(11, 0);

    let inputs = vec![anchor, extra];

    let mut dummy = DummyAccounts::default();
    let mut ctx = ctx_for(&mut dummy);
    let parsed = AnchorAttr::try_utxos(&mut ctx, &inputs).expect("should parse with anchor attr");
    assert_eq!(parsed.anchor_utxo.meta.txid()[0], 10);
    assert_eq!(parsed.others.len(), 1);
}

#[derive(Debug, Accounts)]
struct DummyAccounts<'info> {
    my_account: AccountInfo<'info>,
}

impl<'info> Default for DummyAccounts<'info> {
    fn default() -> Self {
        use arch_program::{pubkey::Pubkey, utxo::UtxoMeta};

        // Leak boxed values to obtain references with 'static lifetime. This
        // is safe in test code because they live until the process exits and
        // we never mutate them concurrently.
        let key: &'static Pubkey = Box::leak(Box::new(Pubkey::default()));
        let lamports: &'static mut u64 = Box::leak(Box::new(0u64));
        let data: &'static mut [u8] = Box::leak(Box::new([0u8; 1]));
        let utxo_meta: &'static UtxoMeta = Box::leak(Box::new(UtxoMeta::from([0u8; 32], 0)));

        let acc_info = AccountInfo::new(
            key, lamports, data, key, // owner
            utxo_meta, false, // is_signer
            false, // is_writable
            false, // is_executable
        );

        Self {
            my_account: acc_info,
        }
    }
}

// -----------------------------------------------------------------------------
// Dynamic anchored Vec<&UtxoInfo> functionality (new feature)
// -----------------------------------------------------------------------------

#[derive(Debug)]
struct ShardedAccounts<'info> {
    shards: Vec<AccountInfo<'info>>, // Vector we will anchor to
}

impl<'info> Default for ShardedAccounts<'info> {
    fn default() -> Self {
        use arch_program::{pubkey::Pubkey, utxo::UtxoMeta};

        let key: &'static Pubkey = Box::leak(Box::new(Pubkey::default()));
        let lamports: &'static mut u64 = Box::leak(Box::new(0u64));
        let data: &'static mut [u8] = Box::leak(Box::new([0u8; 1]));
        let utxo_meta: &'static UtxoMeta = Box::leak(Box::new(UtxoMeta::from([0u8; 32], 0)));

        let acc_info = AccountInfo::new(
            key, lamports, data, key, // owner
            utxo_meta, false, false, false,
        );

        // Build a vector with **runtime** length (not a const generic)
        Self {
            shards: vec![acc_info.clone(), acc_info.clone(), acc_info.clone()],
        }
    }
}

#[derive(Debug, UtxoParser)]
#[utxo_accounts(ShardedAccounts)]
struct AnchoredVecParser {
    // Must match the length of `accounts.shards` (3) – checked at runtime
    #[utxo(anchor = shards, value = 1)]
    shard_utxos: Vec<UtxoInfo>,
}

#[test]
fn anchored_vec_parses_with_matching_len() {
    // three matching UTXOs (value = 1)
    // Use unique txid bytes (40,41,42) to avoid colliding with other tests
    let m0 = create_meta(40, 0);
    let m1 = create_meta(41, 1);
    let m2 = create_meta(42, 2);

    // Register fully-populated info so the value predicate succeeds off-chain
    for meta in [&m0, &m1, &m2] {
        register_test_utxo_info(UtxoInfo {
            meta: (*meta).clone(),
            value: 1,
            ..Default::default()
        });
    }

    let inputs = vec![m0.clone(), m1.clone(), m2.clone()];

    let mut accs = ShardedAccounts::default();
    let mut ctx = ctx_for(&mut accs);
    let parsed = AnchoredVecParser::try_utxos(&mut ctx, &inputs).expect("anchored vec should parse");
    assert_eq!(parsed.shard_utxos.len(), 3);
}

#[test]
fn anchored_vec_fails_when_len_mismatch() {
    // only two UTXOs instead of three
    let m0 = create_meta(50, 0);
    let m1 = create_meta(51, 1);

    for meta in [&m0, &m1] {
        register_test_utxo_info(UtxoInfo {
            meta: (*meta).clone(),
            value: 1,
            ..Default::default()
        });
    }

    let inputs = vec![m0.clone(), m1.clone()];

    let mut accs = ShardedAccounts::default();
    let mut ctx = ctx_for(&mut accs);
    let err = AnchoredVecParser::try_utxos(&mut ctx, &inputs).unwrap_err();
    // Any predicate failure here maps to MissingRequiredUtxo / InvalidUtxoValue / InvalidRunesPresence.
    assert_eq!(
        err,
        ProgramError::Custom(ErrorCode::MissingRequiredUtxo.into())
    );
}

// Provide a dummy Bumps implementation so that ShardedAccounts satisfies the trait bound
impl<'info> anchor_lang::Bumps for ShardedAccounts<'info> {
    type Bumps = ();
}

impl<'info> anchor_lang::Accounts<'info, ()> for ShardedAccounts<'info> {
    fn try_accounts(
        _program_id: &Pubkey,
        _accounts: &mut &'info [AccountInfo<'info>],
        _ix_data: &[u8],
        _bumps: &mut (),
        _reallocs: &mut BTreeSet<Pubkey>,
    ) -> anchor_lang::Result<Self> {
        Ok(Self::default())
    }
}

// Implement the minimal trait set required for ShardedAccounts to satisfy the `Accounts` supertraits.
impl<'info> anchor_lang::ToAccountInfos<'info> for ShardedAccounts<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        self.shards.clone()
    }
}

impl anchor_lang::ToAccountMetas for ShardedAccounts<'_> {
    fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<AccountMeta> {
        Vec::new()
    }
}

impl<'info> anchor_lang::AccountsExit<'info> for ShardedAccounts<'info> {}
