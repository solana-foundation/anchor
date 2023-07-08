extern crate proc_macro;

#[cfg(feature = "idl-build")]
use {anchor_syn::idl::build::gen_idl_print_function_for_constant, quote::quote, syn};

/// A marker attribute used to mark const values that should be included in the
/// generated IDL but functionally does nothing.
#[proc_macro_attribute]
pub fn constant(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    #[cfg(feature = "idl-build")]
    {
        let ts = match syn::parse(input).unwrap() {
            syn::Item::Const(item) => {
                let idl_print = gen_idl_print_function_for_constant(&item);
                quote! {
                    #item
                    #idl_print
                }
            }
            item => quote! {#item},
        };

        return proc_macro::TokenStream::from(quote! {
            #ts
        });
    };

    #[allow(unreachable_code)]
    input
}
