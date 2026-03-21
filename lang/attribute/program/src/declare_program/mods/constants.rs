use anchor_lang_idl::types::{Idl, IdlType};
use quote::{format_ident, quote, ToTokens};

use super::common::{convert_idl_type_to_str, gen_docs};

pub fn gen_constants_mod(idl: &Idl) -> syn::Result<proc_macro2::TokenStream> {
    let constants = idl
        .constants
        .iter()
        .map(|c| {
            let name = format_ident!("{}", c.name);
            let docs = gen_docs(&c.docs);
            let ty_str = convert_idl_type_to_str(&c.ty, true)?;
            let ty = syn::parse_str::<syn::Type>(&ty_str).map_err(|err| {
                syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!("Failed to parse constant type `{ty_str}`: {err}"),
                )
            })?;
            let val = syn::parse_str::<syn::Expr>(&c.value)
                .map_err(|err| {
                    syn::Error::new(
                        proc_macro2::Span::call_site(),
                        format!("Failed to parse constant value `{}`: {err}", c.value),
                    )
                })?
                .to_token_stream();
            let val = match &c.ty {
                IdlType::Bytes => quote! { &#val },
                IdlType::Pubkey => quote!(Pubkey::from_str_const(stringify!(#val))),
                _ => val,
            };

            Ok(quote! {
                #docs
                pub const #name: #ty = #val;
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        /// Program constants.
        pub mod constants {
            use super::*;

            #(#constants)*
        }
    })
}
