#![cfg(feature = "runes")]

use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use arch_program::account::AccountInfo;
use arch_program::program_error::ProgramError;
use arch_program::rune::{RuneAmount, RuneId};
use arch_program::utxo::UtxoMeta;
use saturn_bitcoin_transactions::utxo_info::{UtxoInfo, UtxoInfoTrait};
use saturn_utxo_parser::register_test_utxo_info;
use saturn_utxo_parser::{ErrorCode, TryFromUtxos};
use saturn_utxo_parser_derive::UtxoParser;

mod test_helpers;
use test_helpers::ctx_for;
// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------
fn create_utxo(value: u64, txid_byte: u8, vout: u32) -> UtxoMeta {
    let txid = [txid_byte; 32];
    let meta = UtxoMeta::from(txid, vout);
    let info = UtxoInfo::<saturn_bitcoin_transactions::utxo_info::SingleRuneSet> {
        meta: meta.clone(),
        value,
        ..Default::default()
    };
    register_test_utxo_info(info);
    meta
}

fn create_utxo_with_rune(value: u64, txid_byte: u8, vout: u32, amount: u128) -> UtxoMeta {
    let txid = [txid_byte; 32];
    let meta = UtxoMeta::from(txid, vout);
    let mut info = UtxoInfo::<saturn_bitcoin_transactions::utxo_info::SingleRuneSet> {
        meta: meta.clone(),
        value,
        ..Default::default()
    };
    let rune = RuneAmount {
        id: RuneId::new(999, 0),
        amount,
    };
    info.runes_mut().insert(rune).unwrap();
    register_test_utxo_info(info);
    meta
}

// -----------------------------------------------------------------------------
// Struct using `anchor` attribute
// -----------------------------------------------------------------------------
#[derive(Debug, UtxoParser)]
#[utxo_accounts(DummyAccounts)]
struct Anchored {
    #[utxo(anchor = my_account)]
    anchor: UtxoInfo,

    #[utxo(rest)]
    rest: Vec<UtxoInfo>,
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------
#[test]
fn anchored_utxo_without_runes_succeeds() {
    let anchor = create_utxo(1_000, 1, 0);
    let extra = create_utxo(2_000, 2, 0);

    let inputs = vec![anchor.clone(), extra.clone()];
    let mut dummy = DummyAccounts::default();
    let mut ctx = ctx_for(&mut dummy);
    let parsed = Anchored::try_utxos(&mut ctx, &inputs).expect("should parse when no runes present");
    assert_eq!(parsed.anchor.value, 1_000);
    assert_eq!(parsed.rest.len(), 1);
}

#[test]
fn anchored_utxo_with_runes_fails() {
    let anchor_with_rune = create_utxo_with_rune(1_000, 0, 0, 42);

    // Only the anchored UTXO is provided; there is no fallback candidate that satisfies the
    // `anchor` predicate (which enforces `runes == none`). The parser must therefore fail.
    let inputs = vec![anchor_with_rune];
    let mut dummy = DummyAccounts::default();
    let mut ctx = ctx_for(&mut dummy);
    let err = Anchored::try_utxos(&mut ctx, &inputs).unwrap_err();
    assert_eq!(
        err,
        ProgramError::Custom(ErrorCode::StrictOrderMismatch.into())
    );
}

// -----------------------------------------------------------------------------
// Dummy Accounts implementation (copied from other tests)
// -----------------------------------------------------------------------------
#[derive(Accounts)]
struct DummyAccounts<'info> {
    my_account: AccountInfo<'info>,
}

impl<'info> Default for DummyAccounts<'info> {
    fn default() -> Self {
        use arch_program::pubkey::Pubkey;

        let key: &'static Pubkey = Box::leak(Box::new(Pubkey::default()));
        let lamports: &'static mut u64 = Box::leak(Box::new(0u64));
        let data: &'static mut [u8] = Box::leak(Box::new([0u8; 1]));
        let utxo_meta: &'static UtxoMeta = Box::leak(Box::new(UtxoMeta::from([0u8; 32], 0)));

        let acc_info = AccountInfo::new(key, lamports, data, key, utxo_meta, false, false, false);

        Self {
            my_account: acc_info,
        }
    }
}
