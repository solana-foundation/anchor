#![cfg(feature = "runes")]

use anchor_lang::prelude::*;
use arch_program::account::AccountInfo;
use arch_program::program_error::ProgramError;
use arch_program::rune::{RuneAmount, RuneId};
use arch_program::utxo::UtxoMeta;
use saturn_bitcoin_transactions::utxo_info::{UtxoInfo, UtxoInfoTrait};
use saturn_utxo_parser::register_test_utxo_info;
use saturn_utxo_parser::{ErrorCode, TryFromUtxos};
use saturn_utxo_parser_derive::UtxoParser;
use anchor_lang::Accounts;

mod test_helpers;
use test_helpers::ctx_for;

// Helper to create a plain UTXO
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

// Helper to create utxo with given rune id and amount
fn create_utxo_with_rune(
    value: u64,
    txid_byte: u8,
    vout: u32,
    rune_id: RuneId,
    amount: u128,
) -> UtxoMeta {
    let txid = [txid_byte; 32];
    let meta = UtxoMeta::from(txid, vout);
    let mut info = UtxoInfo::<saturn_bitcoin_transactions::utxo_info::SingleRuneSet> {
        meta: meta.clone(),
        value,
        ..Default::default()
    };
    let rune = RuneAmount {
        id: rune_id,
        amount,
    };
    info.runes_mut().insert(rune).unwrap();
    register_test_utxo_info(info);
    meta
}

// Helper function to avoid const privacy limitations
fn target_rune_id() -> RuneId {
    RuneId::new(777, 0)
}

#[derive(Debug, UtxoParser)]
#[utxo_accounts(DummyAccounts)]
struct ExactRune {
    #[utxo(rune_id = RuneId::new(777, 0), rune_amount = 500)]
    exact: UtxoInfo,
}

// Success path
#[test]
fn exact_rune_success() {
    let matching_utxo = create_utxo_with_rune(1_000, 1, 0, target_rune_id(), 500);
    let mut dummy = DummyAccounts::default();
    let mut ctx = ctx_for(&mut dummy);
    let inputs = vec![matching_utxo];
    let parsed = ExactRune::try_utxos(&mut ctx, &inputs).expect("should parse");
    assert_eq!(parsed.exact.value, 1_000);
}

// Wrong rune id should error
#[test]
fn rune_id_mismatch_error() {
    let wrong_id = RuneId::new(999, 0);
    let utxo = create_utxo_with_rune(1_000, 2, 0, wrong_id, 500);
    let mut dummy = DummyAccounts::default();
    let mut ctx = ctx_for(&mut dummy);
    let inputs = vec![utxo];
    let err = ExactRune::try_utxos(&mut ctx, &inputs).unwrap_err();
    assert_eq!(err, ProgramError::Custom(ErrorCode::InvalidRuneId.into()));
}

// Wrong amount should error
#[test]
fn rune_amount_mismatch_error() {
    let utxo = create_utxo_with_rune(1_000, 3, 0, target_rune_id(), 499);
    let mut dummy = DummyAccounts::default();
    let mut ctx = ctx_for(&mut dummy);
    let inputs = vec![utxo];
    let err = ExactRune::try_utxos(&mut ctx, &inputs).unwrap_err();
    assert_eq!(
        err,
        ProgramError::Custom(ErrorCode::InvalidRuneAmount.into())
    );
}

// ---------------------------------- Dummy Accounts ----------------------------------
#[derive(Debug, Accounts)]
struct DummyAccounts<'info> {
    dummy: AccountInfo<'info>,
}

impl<'info> Default for DummyAccounts<'info> {
    fn default() -> Self {
        use arch_program::pubkey::Pubkey;

        let key: &'static Pubkey = Box::leak(Box::new(Pubkey::default()));
        let lamports: &'static mut u64 = Box::leak(Box::new(0u64));
        let data: &'static mut [u8] = Box::leak(Box::new([0u8; 1]));
        let utxo_meta: &'static UtxoMeta = Box::leak(Box::new(UtxoMeta::from([0u8; 32], 0)));

        let acc_info = AccountInfo::new(key, lamports, data, key, utxo_meta, false, false, false);

        Self { dummy: acc_info }
    }
}
