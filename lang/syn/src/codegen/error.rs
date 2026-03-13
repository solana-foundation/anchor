use crate::Error;
use quote::quote;

pub fn generate(error: Error) -> proc_macro2::TokenStream {
    let error_enum = &error.raw_enum;
    let enum_name = &error.ident;
    // Each arm of the `match` statement for implementing `core::fmt::Display`
    // on the user defined error code.
    let display_variant_dispatch: Vec<proc_macro2::TokenStream> = error
        .raw_enum
        .variants
        .iter()
        .enumerate()
        .map(|(idx, variant)| {
            let ident = &variant.ident;
            let error_code = &error.codes[idx];
            let display_msg = match &error_code.msg {
                None => {
                    quote! {
                        <Self as core::fmt::Debug>::fmt(self, fmt)
                    }
                }
                Some(msg) => {
                    quote! {
                        write!(fmt, #msg)
                    }
                }
            };
            quote! {
                #enum_name::#ident => #display_msg
            }
        })
        .collect();

    // Each arm of the `match` statement for implementing the `name` function
    // on the user defined error code.
    let name_variant_dispatch: Vec<proc_macro2::TokenStream> = error
        .raw_enum
        .variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;
            let ident_name = ident.to_string();
            quote! {
                #enum_name::#ident => alloc::string::ToString::to_string(#ident_name)
            }
        })
        .collect();

    let offset = match &error.args {
        None => quote! { anchor_lang::error::ERROR_CODE_OFFSET},
        Some(args) => {
            let offset = &args.offset;
            quote! { #offset }
        }
    };

    let ret = quote! {
        #[derive(core::fmt::Debug, Clone, Copy)]
        #[repr(u32)]
        #error_enum

        impl #enum_name {
            /// Gets the name of this [#enum_name].
            pub fn name(&self) -> std::string::String {
                match self {
                    #(#name_variant_dispatch),*
                }
            }
        }

        impl From<#enum_name> for u32 {
            fn from(e: #enum_name) -> u32 {
                e as u32 + #offset
            }
        }

        impl From<#enum_name> for anchor_lang::error::Error {
            fn from(error_code: #enum_name) -> anchor_lang::error::Error {
                anchor_lang::error::Error::from(
                    anchor_lang::error::AnchorError {
                        error_name: error_code.name(),
                        error_code_number: error_code.into(),
                        error_msg: alloc::string::ToString::to_string(&error_code),
                        error_origin: None,
                        compared_values: None
                    }
                )
            }
        }

        impl core::fmt::Display for #enum_name {
            fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::result::Result<(), core::fmt::Error> {
                match self {
                    #(#display_variant_dispatch),*
                }
            }
        }
    };

    #[cfg(feature = "idl-build")]
    {
        let idl_print = crate::idl::gen_idl_print_fn_error(&error);
        return quote! {
            #ret
            #idl_print
        };
    };

    #[allow(unreachable_code)]
    ret
}
