use crate::Program;
use heck::CamelCase;
use quote::quote;

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    // Dispatch all global instructions.
    let global_dispatch_arms: Vec<proc_macro2::TokenStream> = program
        .ixs
        .iter()
        .map(|ix| {
            let ix_method_name = &ix.raw_method.sig.ident;
            let ix_name_camel: proc_macro2::TokenStream = ix_method_name
                .to_string()
                .as_str()
                .to_camel_case()
                .parse()
                .expect("Failed to parse ix method name in camel as `TokenStream`");

            quote! {
                instruction::#ix_name_camel::DISCRIMINATOR => {
                    __private::__global::#ix_method_name(
                        program_id,
                        accounts,
                        ix_data,
                    )
                }
            }
        })
        .collect();
    let fallback_fn = gen_fallback(program).unwrap_or(quote! {
        Err(anchor_lang::error::ErrorCode::InstructionFallbackNotFound.into())
    });
    quote! {
        /// Performs method dispatch.
        ///
        /// Each method in an anchor program is uniquely defined by a namespace
        /// and a rust identifier (i.e., the name given to the method). These
        /// two pieces can be combined to creater a method identifier,
        /// specifically, Anchor uses
        ///
        /// Sha256("<namespace>:<rust-identifier>")[..8],
        ///
        /// where the namespace can be one type. "global" for a
        /// regular instruction.
        ///
        /// With this 8 byte identifier, Anchor performs method dispatch,
        /// matching the given 8 byte identifier to the associated method
        /// handler, which leads to user defined code being eventually invoked.
        fn dispatch(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            data: &[u8],
        ) -> anchor_lang::Result<()> {
            // Split the instruction data into the first 8 byte method
            // identifier (sighash) and the serialized instruction data.
            let mut ix_data: &[u8] = data;
            let sighash: [u8; 8] = {
                let mut sighash: [u8; 8] = [0; 8];
                sighash.copy_from_slice(&ix_data[..8]);
                ix_data = &ix_data[8..];
                sighash
            };

            // If the method identifier is the IDL tag, then execute an IDL
            // instruction, injected into all Anchor programs.
            if cfg!(not(feature = "no-idl")) {
                if sighash == anchor_lang::idl::IDL_IX_TAG.to_le_bytes() {
                    return __private::__idl::__idl_dispatch(
                        program_id,
                        accounts,
                        &ix_data,
                    );
                }
            }

            use anchor_lang::Discriminator;
            match sighash {
                #(#global_dispatch_arms)*
                _ => {
                    #fallback_fn
                }
            }
        }
    }
}

pub fn gen_fallback(program: &Program) -> Option<proc_macro2::TokenStream> {
    program.fallback_fn.as_ref().map(|fallback_fn| {
        let program_name = &program.name;
        let method = &fallback_fn.raw_method;
        let fn_name = &method.sig.ident;
        quote! {
            #program_name::#fn_name(program_id, accounts, data)
        }
    })
}
