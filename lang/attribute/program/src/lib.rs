extern crate proc_macro;

use quote::ToTokens;
use syn::parse_macro_input;

/// The `#[program]` attribute defines the module containing all instruction
/// handlers defining all entries into a Solana program.
#[proc_macro_attribute]
pub fn program(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    parse_macro_input!(input as anchor_syn::Program)
        .to_token_stream()
        .into()
}
