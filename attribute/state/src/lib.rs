extern crate proc_macro;

use quote::quote;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn state(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item_struct = parse_macro_input!(input as syn::ItemStruct);

    proc_macro::TokenStream::from(quote! {
        #[account("state")]
        #item_struct
    })
}

/*
        impl<'a, 'b, 'c, 'info> anchor_lang::ProgramStateContext<'a, 'b, 'c, 'info> for #strct {
            fn context() -> Context<'a, 'b, 'c, 'info, ProgramStateAccounts<'info>> {
                // todo
            }
        }
*/
