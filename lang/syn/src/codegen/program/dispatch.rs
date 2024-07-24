use crate::Program;
use heck::CamelCase;
use quote::quote;

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    // Dispatch all global instructions.
    let global_ixs = program.ixs.iter().map(|ix| {
        let ix_method_name = &ix.raw_method.sig.ident;
        let ix_name_camel: proc_macro2::TokenStream = ix_method_name
            .to_string()
            .to_camel_case()
            .parse()
            .expect("Failed to parse ix method name in camel as `TokenStream`");
        let discriminator = quote! { instruction::#ix_name_camel::DISCRIMINATOR };

        quote! {
            if data.starts_with(#discriminator) {
                return __private::__global::#ix_method_name(
                    program_id,
                    accounts,
                    &data[#discriminator.len()..],
                )
            }
        }
    });

    // Generate the event-cpi instruction handler based on whether the `event-cpi` feature is enabled.
    let event_cpi_handler = {
        #[cfg(feature = "event-cpi")]
        quote! {
            // `event-cpi` feature is enabled, dispatch self-cpi instruction
            __private::__events::__event_dispatch(
                program_id,
                accounts,
                &data[anchor_lang::event::EVENT_IX_TAG_LE.len()..]
            )
        }
        #[cfg(not(feature = "event-cpi"))]
        quote! {
            // `event-cpi` feature is not enabled
            Err(anchor_lang::error::ErrorCode::EventInstructionStub.into())
        }
    };

    let fallback_fn = program
        .fallback_fn
        .as_ref()
        .map(|fallback_fn| {
            let program_name = &program.name;
            let fn_name = &fallback_fn.raw_method.sig.ident;
            quote! {
                #program_name::#fn_name(program_id, accounts, data)
            }
        })
        .unwrap_or_else(|| {
            quote! {
                Err(anchor_lang::error::ErrorCode::InstructionFallbackNotFound.into())
            }
        });

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
            #(#global_ixs)*

            // Dispatch IDL instructions
            if data.starts_with(anchor_lang::idl::IDL_IX_TAG_LE) {
                // If the method identifier is the IDL tag, then execute an IDL
                // instruction, injected into all Anchor programs unless they have
                // `no-idl` feature enabled
                #[cfg(not(feature = "no-idl"))]
                return __private::__idl::__idl_dispatch(
                    program_id,
                    accounts,
                    &data[anchor_lang::idl::IDL_IX_TAG_LE.len()..],
                );
                #[cfg(feature = "no-idl")]
                return Err(anchor_lang::error::ErrorCode::IdlInstructionStub.into());
            }

            // Dispatch Event CPI instruction
            if data.starts_with(anchor_lang::event::EVENT_IX_TAG_LE) {
                return #event_cpi_handler;
            }

            #fallback_fn
        }
    }
}
