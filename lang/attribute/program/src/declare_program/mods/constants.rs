use anchor_lang_idl::types::{Idl, IdlType};
use quote::{format_ident, quote, ToTokens};

use super::common::{convert_idl_type_to_str, gen_docs};

pub fn gen_constants_mod(idl: &Idl) -> proc_macro2::TokenStream {
    let constants = idl.constants.iter().map(|c| {
        let name = format_ident!("{}", c.name);
        let docs = gen_docs(&c.docs);
        let ty = syn::parse_str::<syn::Type>(&convert_idl_type_to_str(&c.ty, true)).unwrap();
        let val = syn::parse_str::<syn::Expr>(&c.value)
            .unwrap()
            .to_token_stream();
        let val = match &c.ty {
            IdlType::Bytes => quote! { &#val },
            IdlType::Pubkey => quote!(Pubkey::from_str_const(stringify!(#val))),
            _ => val,
        };

        quote! {
            #docs
            pub const #name: #ty = #val;
        }
    });

    quote! {
        /// Program constants.
        pub mod constants {
            use super::*;

            #(#constants)*
        }
    }
}
