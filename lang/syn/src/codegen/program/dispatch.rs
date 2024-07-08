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

    let event_cpi_handler = generate_event_cpi_handler();

    quote! {
        /// Performs method dispatch.
        ///
        /// Each method in an anchor program is uniquely defined by a namespace
        /// and a rust identifier (i.e., the name given to the method). These
        /// two pieces can be combined to create a method identifier,
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
        fn dispatch<'info>(
            program_id: &Pubkey,
            accounts: &'info [AccountInfo<'info>],
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

            match sighash {
                #(#global_dispatch_arms)*
                anchor_lang::idl::IDL_IX_TAG_LE => {
                    // If the method identifier is the IDL tag, then execute an IDL
                    // instruction, injected into all Anchor programs unless they have
                    // no-idl enabled
                    #[cfg(not(feature = "no-idl"))]
                    {
                        __private::__idl::__idl_dispatch(
                            program_id,
                            accounts,
                            &ix_data,
                        )
                    }
                    #[cfg(feature = "no-idl")]
                    {
                        Err(anchor_lang::error::ErrorCode::IdlInstructionStub.into())
                    }
                }
                anchor_lang::event::EVENT_IX_TAG_LE => {
                    #event_cpi_handler
                }
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

/// Generate the event-cpi instruction handler based on whether the `event-cpi` feature is enabled.
pub fn generate_event_cpi_handler() -> proc_macro2::TokenStream {
    #[cfg(feature = "event-cpi")]
    quote! {
        // `event-cpi` feature is enabled, dispatch self-cpi instruction
        __private::__events::__event_dispatch(program_id, accounts, &ix_data)
    }
    #[cfg(not(feature = "event-cpi"))]
    quote! {
        // `event-cpi` feature is not enabled
        Err(anchor_lang::error::ErrorCode::EventInstructionStub.into())
    }
}
