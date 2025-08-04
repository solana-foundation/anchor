use anchor_lang_idl::types::Idl;
use quote::{format_ident, quote};

pub fn gen_utils_mod(idl: &Idl) -> proc_macro2::TokenStream {
    let account = gen_account(idl);
    let event = gen_event(idl);

    quote! {
        /// Program utilities.
        pub mod utils {
            use super::*;

            #account
            #event
        }
    }
}

fn gen_account(idl: &Idl) -> proc_macro2::TokenStream {
    let variants = idl
        .accounts
        .iter()
        .map(|acc| format_ident!("{}", acc.name))
        .map(|name| quote! { #name(#name) });
    let if_statements = idl.accounts.iter().map(|acc| {
        let name = format_ident!("{}", acc.name);
        quote! {
            if value.starts_with(#name::DISCRIMINATOR) {
                return #name::try_deserialize_unchecked(&mut &value[..])
                    .map(Self::#name)
                    .map_err(Into::into)
            }
        }
    });

    quote! {
        /// An enum that includes all accounts of the declared program as a tuple variant.
        ///
        /// See [`Self::try_from_bytes`] to create an instance from bytes.
        pub enum Account {
            #(#variants,)*
        }

        impl Account {
            /// Try to create an account based on the given bytes.
            ///
            /// This method returns an error if the discriminator of the given bytes don't match
            /// with any of the existing accounts, or if the deserialization fails.
            pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
                Self::try_from(bytes)
            }
        }

        impl TryFrom<&[u8]> for Account {
            type Error = anchor_lang::error::Error;

            fn try_from(value: &[u8]) -> Result<Self> {
                #(#if_statements)*
                Err(ProgramError::InvalidArgument.into())
            }
        }
    }
}

fn gen_event(idl: &Idl) -> proc_macro2::TokenStream {
    let variants = idl
        .events
        .iter()
        .map(|ev| format_ident!("{}", ev.name))
        .map(|name| quote! { #name(#name) });
    let if_statements = idl.events.iter().map(|ev| {
        let name = format_ident!("{}", ev.name);
        quote! {
            if value.starts_with(#name::DISCRIMINATOR) {
                return #name::try_from_slice(&value[#name::DISCRIMINATOR.len()..])
                    .map(Self::#name)
                    .map_err(Into::into)
            }
        }
    });

    quote! {
        /// An enum that includes all events of the declared program as a tuple variant.
        ///
        /// See [`Self::try_from_bytes`] to create an instance from bytes.
        pub enum Event {
            #(#variants,)*
        }

        impl Event {
            /// Try to create an event based on the given bytes.
            ///
            /// This method returns an error if the discriminator of the given bytes don't match
            /// with any of the existing events, or if the deserialization fails.
            pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
                Self::try_from(bytes)
            }
        }

        impl TryFrom<&[u8]> for Event {
            type Error = anchor_lang::error::Error;

            fn try_from(value: &[u8]) -> Result<Self> {
                #(#if_statements)*
                Err(ProgramError::InvalidArgument.into())
            }
        }
    }
}
