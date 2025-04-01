use heck::SnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::{
    common::{gen_print_section, get_idl_module_path, get_no_docs},
    defined::gen_idl_type,
};
use crate::parser::docs;

pub fn gen_idl_print_fn_constant(item: &syn::ItemConst) -> TokenStream {
    let idl = get_idl_module_path();
    let no_docs = get_no_docs();

    let name = item.ident.to_string();
    let expr = &item.expr;
    let fn_name = format_ident!("__anchor_private_print_idl_const_{}", name.to_snake_case());

    let docs = match docs::parse(&item.attrs) {
        Some(docs) if !no_docs => quote! { vec![#(#docs.into()),*] },
        _ => quote! { vec![] },
    };

    let fn_body = match gen_idl_type(&item.ty, &[]) {
        Ok((ty, _)) => {
            // Check if the type is a string directly
            let is_string_type = match &*item.ty {
                syn::Type::Path(type_path) => {
                    // Check that the path consists of a single segment with the identifier "string"
                    type_path.path.segments.len() == 1 
                        && type_path.path.segments[0].ident == "string"
                }
                _ => false,
            };
            
            // Use different formatting based on the type
            let value_format = if is_string_type {
                // For string types, use Display formatting to avoid extra quotes
                quote! { format!("{}", #expr) }
            } else {
                // For other types, continue using Debug formatting
                quote! { format!("{:?}", #expr) }
            };
            
            gen_print_section(
                "const",
                quote! {
                    #idl::IdlConst {
                        name: #name.into(),
                        docs: #docs,
                        ty: #ty,
                        value: #value_format,
                    }
                },
            )
        },
        _ => quote! {},
    };

    quote! {
        #[test]
        pub fn #fn_name() {
            #fn_body
        }
    }
}
