use crate::Error;
use quote::quote;

pub fn generate(error: Error) -> proc_macro2::TokenStream {
    let error_enum = error.raw_enum;
    let enum_name = &error.ident;
    quote! {
        type Result<T> = std::result::Result<T, Error>;

        #[derive(thiserror::Error, Debug)]
        pub enum Error {
            #[error(transparent)]
            ProgramError(#[from] ProgramError),
            #[error("{0:?}")]
            ErrorCode(#[from] #enum_name),
        }

        #[derive(Debug, Clone, Copy)]
        #[repr(u32)]
        #error_enum

        impl std::fmt::Display for #enum_name {
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                <Self as std::fmt::Debug>::fmt(self, fmt)
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
    }
}
