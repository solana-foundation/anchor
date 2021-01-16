extern crate proc_macro;

use anchor_syn::codegen::error as error_codegen;
use anchor_syn::parser::error as error_parser;
use syn::parse_macro_input;

/// Generates an error type from an error code enum.
#[proc_macro_attribute]
pub fn error(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut error_enum = parse_macro_input!(input as syn::ItemEnum);
    let error = error_codegen::generate(error_parser::parse(&mut error_enum));
    proc_macro::TokenStream::from(error)
}
