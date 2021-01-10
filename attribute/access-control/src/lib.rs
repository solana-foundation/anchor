extern crate proc_macro;

use quote::quote;
use syn::parse_macro_input;

/// Executes the given access control method before running the decorated
/// instruction handler. Any method in scope of the attribute can be invoked
/// with any arguments from the associated instruction handler.
#[proc_macro_attribute]
pub fn access_control(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let access_control: proc_macro2::TokenStream = args.to_string().parse().unwrap();

    let item_fn = parse_macro_input!(input as syn::ItemFn);

    let fn_vis = item_fn.vis;
    let fn_sig = item_fn.sig;
    let fn_block = item_fn.block;

    let fn_stmts = fn_block.stmts;

    proc_macro::TokenStream::from(quote! {
        #fn_vis #fn_sig {

            #access_control?;

            #(#fn_stmts)*
        }
    })
}
