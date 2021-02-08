extern crate proc_macro;

use quote::quote;
use syn::parse_macro_input;

/// The `#[state]` attribute defines the program's state struct, i.e., the
/// program's global account singleton giving the program the illusion of state.
#[proc_macro_attribute]
pub fn state(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item_struct = parse_macro_input!(input as syn::ItemStruct);

    proc_macro::TokenStream::from(quote! {
        #[account]
        #item_struct
    })
}
