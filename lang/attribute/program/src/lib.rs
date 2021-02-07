extern crate proc_macro;

use anchor_syn::codegen::program as program_codegen;
use anchor_syn::parser::program as program_parser;
use syn::parse_macro_input;

/// The `#[program]` attribute defines the module containing all instruction
/// handlers defining all entries into a Solana program.
#[proc_macro_attribute]
pub fn program(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let program_mod = parse_macro_input!(input as syn::ItemMod);
    let code = program_codegen::generate(program_parser::parse(program_mod));
    proc_macro::TokenStream::from(code)
}
