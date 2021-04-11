use crate::Error;
use quote::quote;

pub fn generate(error: Error) -> proc_macro2::TokenStream {
    let error_enum = &error.raw_enum;
    let enum_name = &error.ident;
    // Each arm of the `match` statement for implementing `std::fmt::Display`
    // on the user defined error code.
    let variant_dispatch: Vec<proc_macro2::TokenStream> = error
        .raw_enum
        .variants
        .iter()
        .enumerate()
        .map(|(idx, variant)| {
            let ident = &variant.ident;
            let error_code = &error.codes[idx];
            let msg = match &error_code.msg {
                None => {
                    quote! {
                        <Self as std::fmt::Debug>::fmt(self, fmt)
                    }
                }
                Some(msg) => {
                    quote! {
                        write!(fmt, #msg)
                    }
                }
            };
            quote! {
                #enum_name::#ident => #msg
            }
        })
        .collect();

    quote! {
        /// Anchor generated Result to be used as the return type for the
        /// program.
        pub type Result<T> = std::result::Result<T, Error>;

        /// Anchor generated error allowing one to easily return a
        /// `ProgramError` or a custom, user defined error code by utilizing
        /// its `From` implementation.
        #[derive(thiserror::Error, Debug)]
        pub enum Error {
            #[error(transparent)]
            ProgramError(#[from] ProgramError),
            #[error(transparent)]
            ErrorCode(#[from] #enum_name),
        }

        #[derive(Debug, Clone, Copy)]
        #[repr(u32)]
        #error_enum

        impl std::fmt::Display for #enum_name {
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                match self {
                    #(#variant_dispatch),*
                }
            }
        }

        impl std::error::Error for #enum_name {}

        impl std::convert::From<Error> for ProgramError {
            fn from(e: Error) -> ProgramError {
            // Errors 0-100 are reserved for the framework.
            let error_offset = 100u32;
                match e {
                    Error::ProgramError(e) => e,
                    Error::ErrorCode(c) => ProgramError::Custom(c as u32 + error_offset),
                }
            }
        }

        impl std::convert::From<#enum_name> for ProgramError {
            fn from(e: #enum_name) -> ProgramError {
                let err: Error = e.into();
                err.into()
            }
        }
    }
}
