use anchor_lang_idl::types::{Idl, IdlType};
use quote::{format_ident, quote, ToTokens};

use super::common::convert_idl_type_to_str;

pub fn gen_constants_mod(idl: &Idl) -> proc_macro2::TokenStream {
    let constants = idl.constants.iter().map(|c| {
        let name = format_ident!("{}", c.name);
        let ty = match &c.ty {
            IdlType::String => quote!(&str),
            _ => parse_expr_ts(&convert_idl_type_to_str(&c.ty)),
        };
        let val = parse_expr_ts(&c.value);

        // TODO: Docs
        quote! { pub const #name: #ty = #val; }
    });

    quote! {
        /// Program constants.
        pub mod constants {
            #(#constants)*
        }
    }
}

fn parse_expr_ts(s: &str) -> proc_macro2::TokenStream {
    syn::parse_str::<syn::Expr>(s).unwrap().to_token_stream()
}
