extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, ItemStruct,
};

/// Attribute macro for defining Anchor accounts structs.
///
/// This macro is equivalent to `#[derive(Accounts)]` but as an attribute macro,
/// allowing future enhancements for compile-time safety checks.
///
/// # Example
///
/// ```ignore
/// #[accounts]
/// pub struct Transfer<'info> {
///     #[account(mut)]
///     pub from: Account<'info, TokenAccount>,
///     pub to: Account<'info, TokenAccount>,
/// }
/// ```
///
/// Currently this is functionally equivalent to:
/// ```ignore
/// #[derive(Accounts)]
/// pub struct Transfer<'info> {
///     #[account(mut)]
///     pub from: Account<'info, TokenAccount>,
///     pub to: Account<'info, TokenAccount>,
/// }
/// ```
#[proc_macro_attribute]
pub fn accounts(args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse instruction args if any
    let instr_args = if !args.is_empty() {
        Some(parse_macro_input!(args as InstructionArgs))
    } else {
        None
    };

    let input_struct = parse_macro_input!(input as ItemStruct);

    // Generate the instruction attribute if present
    let instruction_attr = instr_args.map(|args| {
        let args_tokens = args.args;
        quote! { #[instruction(#args_tokens)] }
    });

    // Output the struct with #[derive(Accounts)]
    let output = quote! {
        #instruction_attr
        #[derive(::anchor_lang::Accounts)]
        #input_struct
    };

    output.into()
}

struct InstructionArgs {
    args: proc_macro2::TokenStream,
}

impl Parse for InstructionArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let args = input.parse::<proc_macro2::TokenStream>()?;
        Ok(Self { args })
    }
}
