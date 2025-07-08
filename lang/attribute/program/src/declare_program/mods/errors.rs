use anchor_lang_idl::types::Idl;
use quote::{format_ident, quote};

pub fn gen_errors_mod(idl: &Idl) -> proc_macro2::TokenStream {
    let errors = idl.errors.iter().map(|e| {
        let name = format_ident!("{}", e.name);
        quote! {
            #name,
        }
    });

    if errors.len() == 0 {
        return quote! {
            /// Program error type definitions.
            pub mod errors {
            }
        };
    }

    quote! {
        /// Program error type definitions.
        pub mod errors {

            #[anchor_lang::error_code]
            pub enum ProgramError {
                #(#errors)*
            }
        }
    }
}
