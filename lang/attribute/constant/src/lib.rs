extern crate proc_macro;

/// A marker attribute used to mark const values that should be included in the
/// generated IDL but functionally does nothing.
#[proc_macro_attribute]
pub fn constant(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    #[cfg(feature = "idl-build")]
    {
        use quote::quote;

        let input_clone = input.clone();
        let ts = match syn::parse(input_clone) {
            syn::Item::Const(item) => {
                let idl_print = anchor_syn::idl::gen_idl_print_fn_constant(&item);
                quote! {
                    #item
                    #idl_print
                }
            }
            item => quote! {#item},
            Err(err) => return err.to_compile_error().into(),
        };

        return proc_macro::TokenStream::from(quote! {
            #ts
        });
    };

    #[allow(unreachable_code)]
    input
}
