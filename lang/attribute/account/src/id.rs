//! Copied from solana/sdk/macro so that Anchor programs don't need to specify
//! `solana_program` as an additional crate dependency, but instead can access
//! it via `anchor_lang::declare_id`.
//!
//! Convenience macro to declare a static public key and functions to interact with it
//!
//! Input: a single literal base58 string representation of a program's id

extern crate proc_macro;

use proc_macro2::{Delimiter, Span, TokenTree};
use quote::{quote, ToTokens};
use std::convert::TryFrom;
use syn::{
    bracketed,
    parse::{Parse, ParseStream, Result},
    punctuated::Punctuated,
    token::Bracket,
    Expr, Ident, LitByte, LitStr, Path, Token,
};

fn parse_id(
    input: ParseStream,
    pubkey_type: proc_macro2::TokenStream,
) -> Result<proc_macro2::TokenStream> {
    let id = if input.peek(syn::LitStr) {
        let id_literal: LitStr = input.parse()?;
        parse_pubkey(&id_literal, &pubkey_type)?
    } else {
        let expr: Expr = input.parse()?;
        quote! { #expr }
    };

    if !input.is_empty() {
        let stream: proc_macro2::TokenStream = input.parse()?;
        return Err(syn::Error::new_spanned(stream, "unexpected token"));
    }
    Ok(id)
}

fn id_to_tokens(
    id: &proc_macro2::TokenStream,
    pubkey_type: proc_macro2::TokenStream,
    tokens: &mut proc_macro2::TokenStream,
) {
    tokens.extend(quote! {
        /// The static program ID
        pub static ID: #pubkey_type = #id;

        /// Confirms that a given pubkey is equivalent to the program ID
        pub fn check_id(id: &#pubkey_type) -> bool {
            id == &ID
        }

        /// Returns the program ID
        pub fn id() -> #pubkey_type {
            ID
        }

        #[cfg(test)]
        #[test]
        fn test_id() {
            assert!(check_id(&id()));
        }
    });
}

fn deprecated_id_to_tokens(
    id: &proc_macro2::TokenStream,
    pubkey_type: proc_macro2::TokenStream,
    tokens: &mut proc_macro2::TokenStream,
) {
    tokens.extend(quote! {
        /// The static program ID
        pub static ID: #pubkey_type = #id;

        /// Confirms that a given pubkey is equivalent to the program ID
        #[deprecated()]
        pub fn check_id(id: &#pubkey_type) -> bool {
            id == &ID
        }

        /// Returns the program ID
        #[deprecated()]
        pub fn id() -> #pubkey_type {
            ID
        }

        #[cfg(test)]
        #[test]
            fn test_id() {
            #[allow(deprecated)]
            assert!(check_id(&id()));
        }
    });
}

pub struct Id(proc_macro2::TokenStream);

impl Parse for Id {
    fn parse(input: ParseStream) -> Result<Self> {
        parse_id(
            input,
            quote! { anchor_lang::solana_program::pubkey::Pubkey },
        )
        .map(Self)
    }
}

impl ToTokens for Id {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        id_to_tokens(
            &self.0,
            quote! { anchor_lang::solana_program::pubkey::Pubkey },
            tokens,
        )
    }
}

struct IdDeprecated(proc_macro2::TokenStream);

impl Parse for IdDeprecated {
    fn parse(input: ParseStream) -> Result<Self> {
        parse_id(
            input,
            quote! { anchor_lang::solana_program::pubkey::Pubkey },
        )
        .map(Self)
    }
}

impl ToTokens for IdDeprecated {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        deprecated_id_to_tokens(
            &self.0,
            quote! { anchor_lang::solana_program::pubkey::Pubkey },
            tokens,
        )
    }
}

struct ProgramSdkId(proc_macro2::TokenStream);
impl Parse for ProgramSdkId {
    fn parse(input: ParseStream) -> Result<Self> {
        parse_id(
            input,
            quote! { anchor_lang::solana_program::pubkey::Pubkey },
        )
        .map(Self)
    }
}

impl ToTokens for ProgramSdkId {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        id_to_tokens(
            &self.0,
            quote! { anchor_lang::solana_program::pubkey::Pubkey },
            tokens,
        )
    }
}

struct ProgramSdkIdDeprecated(proc_macro2::TokenStream);
impl Parse for ProgramSdkIdDeprecated {
    fn parse(input: ParseStream) -> Result<Self> {
        parse_id(
            input,
            quote! { anchor_lang::solana_program::pubkey::Pubkey },
        )
        .map(Self)
    }
}

impl ToTokens for ProgramSdkIdDeprecated {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        deprecated_id_to_tokens(
            &self.0,
            quote! { anchor_lang::solana_program::pubkey::Pubkey },
            tokens,
        )
    }
}

#[allow(dead_code)] // `respan` may be compiled out
struct RespanInput {
    to_respan: Path,
    respan_using: Span,
}

impl Parse for RespanInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let to_respan: Path = input.parse()?;
        let _comma: Token![,] = input.parse()?;
        let respan_tree: TokenTree = input.parse()?;
        match respan_tree {
            TokenTree::Group(g) if g.delimiter() == Delimiter::None => {
                let ident: Ident = syn::parse2(g.stream())?;
                Ok(RespanInput {
                    to_respan,
                    respan_using: ident.span(),
                })
            }
            val => Err(syn::Error::new_spanned(
                val,
                "expected None-delimited group",
            )),
        }
    }
}

fn parse_pubkey(
    id_literal: &LitStr,
    pubkey_type: &proc_macro2::TokenStream,
) -> Result<proc_macro2::TokenStream> {
    let id_vec = bs58::decode(id_literal.value())
        .into_vec()
        .map_err(|_| syn::Error::new_spanned(&id_literal, "failed to decode base58 string"))?;
    let id_array = <[u8; 32]>::try_from(<&[u8]>::clone(&&id_vec[..])).map_err(|_| {
        syn::Error::new_spanned(
            &id_literal,
            format!("pubkey array is not 32 bytes long: len={}", id_vec.len()),
        )
    })?;
    let bytes = id_array.iter().map(|b| LitByte::new(*b, Span::call_site()));
    Ok(quote! {
        #pubkey_type::new_from_array(
            [#(#bytes,)*]
        )
    })
}

struct Pubkeys {
    method: Ident,
    num: usize,
    pubkeys: proc_macro2::TokenStream,
}
impl Parse for Pubkeys {
    fn parse(input: ParseStream) -> Result<Self> {
        let pubkey_type = quote! {
            anchor_lang::solana_program::pubkey::Pubkey
        };

        let method = input.parse()?;
        let _comma: Token![,] = input.parse()?;
        let (num, pubkeys) = if input.peek(syn::LitStr) {
            let id_literal: LitStr = input.parse()?;
            (1, parse_pubkey(&id_literal, &pubkey_type)?)
        } else if input.peek(Bracket) {
            let pubkey_strings;
            bracketed!(pubkey_strings in input);
            let punctuated: Punctuated<LitStr, Token![,]> =
                Punctuated::parse_terminated(&pubkey_strings)?;
            let mut pubkeys: Punctuated<proc_macro2::TokenStream, Token![,]> = Punctuated::new();
            for string in punctuated.iter() {
                pubkeys.push(parse_pubkey(string, &pubkey_type)?);
            }
            (pubkeys.len(), quote! {#pubkeys})
        } else {
            let stream: proc_macro2::TokenStream = input.parse()?;
            return Err(syn::Error::new_spanned(stream, "unexpected token"));
        };

        Ok(Pubkeys {
            method,
            num,
            pubkeys,
        })
    }
}

impl ToTokens for Pubkeys {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Pubkeys {
            method,
            num,
            pubkeys,
        } = self;

        let pubkey_type = quote! {
            anchor_lang::solana_program::pubkey::Pubkey
        };
        if *num == 1 {
            tokens.extend(quote! {
                pub fn #method() -> #pubkey_type {
                    #pubkeys
                }
            });
        } else {
            tokens.extend(quote! {
                pub fn #method() -> ::std::vec::Vec<#pubkey_type> {
                    vec![#pubkeys]
                }
            });
        }
    }
}
