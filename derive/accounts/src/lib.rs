extern crate proc_macro;

use anchor_syn::codegen::accounts as accounts_codegen;
use anchor_syn::parser::accounts as accounts_parser;
use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Implements an `Accounts` deserializer on the given struct, applying any
/// constraints specified via `#[account]` attributes.
#[proc_macro_derive(Accounts, attributes(account))]
pub fn derive_anchor_deserialize(item: TokenStream) -> TokenStream {
    let strct = parse_macro_input!(item as syn::ItemStruct);
    let tts = accounts_codegen::generate(accounts_parser::parse(&strct));
    proc_macro::TokenStream::from(tts)
}
