use anchor_lang_idl::types::Idl;
use quote::{format_ident, quote};

pub fn gen_errors_mod(idl: &Idl) -> proc_macro2::TokenStream {
    let errors = idl.errors.iter().map(|e| {
        let name = format_ident!("{}", e.name);
        let code = e.code;
        quote! {
            #name = #code,
        }
    });

    if errors.len() == 0 {
        return quote! {
            /// Program error type definitions.
            #[cfg(not(feature = "idl-build"))]
            pub mod errors {
            }
        };
    }

    quote! {
        /// Program error type definitions.
        #[cfg(not(feature = "idl-build"))]
        pub mod errors {

            #[anchor_lang::error_code(offset = 0)]
            pub enum ProgramError {
                #(#errors)*
            }
        }
    }
}
