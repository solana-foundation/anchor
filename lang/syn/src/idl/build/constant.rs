use heck::SnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::{
    common::{gen_print_section, get_idl_module_path},
    defined::gen_idl_type,
};

pub fn gen_idl_print_fn_constant(item: &syn::ItemConst) -> TokenStream {
    let idl = get_idl_module_path();

    let name = item.ident.to_string();
    let expr = &item.expr;
    let fn_name = format_ident!("__anchor_private_print_idl_const_{}", name.to_snake_case());

    let fn_body = match gen_idl_type(&item.ty, &[]) {
        Ok((ty, _)) => gen_print_section(
            "const",
            quote! {
                #idl::IdlConst {
                    name: #name.into(),
                    ty: #ty,
                    value: format!("{:?}", #expr),
                }
            },
        ),
        _ => quote! {},
    };

    quote! {
        #[test]
        pub fn #fn_name() {
            #fn_body
        }
    }
}
