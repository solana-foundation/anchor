use saturn_bitcoin_transactions::utxo_info::UtxoInfo;
use saturn_utxo_parser_derive::UtxoParser;
use anchor_lang::prelude::*;

// Dummy Accounts type deliberately missing the `missing` field referenced by the
// `anchor` attribute below. This should trigger a compile-time error.
#[derive(Debug, Accounts)]
struct DummyAccounts<'info> {
    acc: AccountInfo<'info>,
}

#[derive(UtxoParser)]
#[utxo_accounts(DummyAccounts)]
struct MissingAnchor {
    // The `anchor = missing` identifier does not exist on `DummyAccounts`; the macro
    // expansion will therefore reference a non-existent field and the compiler
    // must emit an error.
    #[utxo(anchor = missing)]
    utxo: UtxoInfo,
}

fn main() {}
