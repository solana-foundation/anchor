extern crate proc_macro;

#[cfg(feature = "lazy-account")]
mod lazy;

use proc_macro::TokenStream;
use proc_macro2::{TokenStream as TokenStream2};
use quote::quote;

fn gen_borsh_serialize(_input: TokenStream) -> TokenStream2 {
    quote! { #[derive(::borsh::BorshSerialize )]}
}

#[proc_macro_derive(AnchorSerialize, attributes(borsh_skip))]
pub fn anchor_serialize(input: TokenStream) -> TokenStream {
    #[cfg(not(feature = "idl-build"))]
    let ret = gen_borsh_serialize(input);
    #[cfg(feature = "idl-build")]
    let ret = gen_borsh_serialize(input.clone());

    #[cfg(feature = "idl-build")]
    {
        use anchor_syn::idl::*;
        use quote::quote;

        let idl_build_impl = match syn::parse(input).unwrap() {
            Item::Struct(item) => impl_idl_build_struct(&item),
            Item::Enum(item) => impl_idl_build_enum(&item),
            Item::Union(item) => impl_idl_build_union(&item),
            // Derive macros can only be defined on structs, enums, and unions.
            _ => unreachable!(),
        };

        return TokenStream::from(quote! {
            #ret
            #idl_build_impl
        });
    };

    #[allow(unreachable_code)]
    TokenStream::from(ret)
}

fn gen_borsh_deserialize(_: TokenStream) -> TokenStream2 {
    quote! { #[derive(::borsh::BorshDeserialize )]}
}

#[proc_macro_derive(AnchorDeserialize, attributes(borsh_skip, borsh_init))]
pub fn borsh_deserialize(input: TokenStream) -> TokenStream {
    #[cfg(feature = "lazy-account")]
    {
        let deser = gen_borsh_deserialize(input.clone());
        let lazy = lazy::gen_lazy(input).unwrap_or_else(|e| e.to_compile_error());
        quote::quote! {
            #deser
            #lazy
        }
        .into()
    }
    #[cfg(not(feature = "lazy-account"))]
    gen_borsh_deserialize(input).into()
}

#[cfg(feature = "lazy-account")]
#[proc_macro_derive(Lazy)]
pub fn lazy(input: TokenStream) -> TokenStream {
    lazy::gen_lazy(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
