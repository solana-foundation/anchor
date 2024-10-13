use anchor_lang_idl::types::{Idl, IdlType};
use quote::{format_ident, quote, ToTokens};

use super::common::{convert_idl_type_to_syn_type, gen_docs};

pub fn gen_constants_mod(idl: &Idl) -> proc_macro2::TokenStream {
    let constants = idl.constants.iter().map(|c| {
        let name = format_ident!("{}", c.name);
        let docs = gen_docs(&c.docs);
        let val = syn::parse_str::<syn::Expr>(&c.value)
            .unwrap()
            .to_token_stream();
        let (ty, val) = match &c.ty {
            IdlType::Bytes => (quote!(&[u8]), quote! { &#val }),
            IdlType::String => (quote!(&str), val),
            _ => (convert_idl_type_to_syn_type(&c.ty).to_token_stream(), val),
        };

        quote! {
            #docs
            pub const #name: #ty = #val;
        }
    });

    quote! {
        /// Program constants.
        pub mod constants {
            #(#constants)*
        }
    }
}
