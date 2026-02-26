use crate::Program;
use heck::CamelCase;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    // Dispatch all global instructions.
    let global_ixs = program.ixs.iter().map(|ix| {
        let ix_method_name = &ix.raw_method.sig.ident;
        let ix_name_camel: proc_macro2::TokenStream = ix_method_name
            .to_string()
            .to_camel_case()
            .parse()
            .expect("Failed to parse ix method name in camel as `TokenStream`");
        let ix_span = ix.raw_method.span();
        let discriminator = quote! { instruction::#ix_name_camel::DISCRIMINATOR };
        let spanned_method_name = quote_spanned! { ix_span => #ix_method_name };
        let ix_cfgs = &ix.cfgs;

        quote! {
            #(#ix_cfgs)*
            if data.starts_with(#discriminator) {
                return __private::__global::#spanned_method_name(
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
            let fallback_span = fallback_fn.raw_method.span();
            let spanned_fn_name = quote_spanned! { fallback_span => #fn_name };
            quote! {
                #program_name::#spanned_fn_name(program_id, accounts, data)
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
        /// Each instruction's discriminator is checked until the given instruction data starts with
        /// the current discriminator.
        ///
        /// If a match is found, the instruction handler is called using the given instruction data
        /// excluding the prepended discriminator bytes.
        ///
        /// If no match is found, the fallback function is executed if it exists, or an error is
        /// returned if it doesn't exist.
        fn dispatch<'info>(
            program_id: &Pubkey,
            accounts: &'info [AccountInfo<'info>],
            data: &[u8],
        ) -> anchor_lang::Result<()> {
            #(#global_ixs)*

            // Legacy IDL instructions have been removed in favor of Program Metadata
            // No IDL instructions are injected into programs anymore

            // Dispatch Event CPI instruction
            if data.starts_with(anchor_lang::event::EVENT_IX_TAG_LE) {
                return #event_cpi_handler;
            }

            #fallback_fn
        }
    }
}
