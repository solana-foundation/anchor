use crate::codegen::program::common::*;
use crate::Program;
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;

// Generate non-inlined wrappers for each instruction handler, since Solana's
// BPF max stack size can't handle reasonable sized dispatch trees without doing
// so.
pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    let program_name = &program.name;

    let event_cpi_mod = generate_event_cpi_mod();

    let non_inlined_handlers: Vec<proc_macro2::TokenStream> = program
        .ixs
        .iter()
        .map(|ix| {
            let ix_arg_names: Vec<&syn::Ident> = ix.args.iter().map(|arg| &arg.name).collect();
            let ix_method_name = &ix.raw_method.sig.ident;
            let ix_method_name_str = ix_method_name.to_string();
            let ix_span = ix.raw_method.span();
            let ix_name = generate_ix_variant_name(&ix_method_name_str);
            let variant_arm = generate_ix_variant_spanned(&ix_method_name_str, &ix.args, ix_span);
            let ix_name_log = format!("Instruction: {ix_name}");
            let anchor = &ix.anchor_ident;
            let ret_type = &ix.returns.ty.to_token_stream();
            let cfgs = &ix.cfgs;
            let ix_span = ix.raw_method.span();
            let maybe_set_return_data = match ret_type.to_string().as_str() {
                "()" => quote! {},
                _ => quote! {
                    let mut return_data = Vec::with_capacity(256);
                    result.serialize(&mut return_data).unwrap();
                    anchor_lang::solana_program::program::set_return_data(&return_data);
                },
            };
            let spanned_fn_name = quote_spanned! { ix_span => #ix_method_name };
            quote! {
                #(#cfgs)*
                #[inline(never)]
                pub fn #spanned_fn_name<'info>(
                    __program_id: &Pubkey,
                    __accounts: &'info[AccountInfo<'info>],
                    __ix_data: &[u8],
                ) -> anchor_lang::Result<()> {
                    #[cfg(not(feature = "no-log-ix-name"))]
                    anchor_lang::prelude::msg!(#ix_name_log);

                    // Deserialize data.
                    let ix = instruction::#ix_name::deserialize(&mut &__ix_data[..])
                        .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
                    let instruction::#variant_arm = ix;

                    // Bump collector.
                    let mut __bumps = <#anchor as anchor_lang::Bumps>::Bumps::default();

                    let mut __reallocs = std::collections::BTreeSet::new();

                    // Deserialize accounts.
                    let mut __remaining_accounts: &[AccountInfo] = __accounts;
                    let mut __accounts = #anchor::try_accounts(
                        __program_id,
                        &mut __remaining_accounts,
                        __ix_data,
                        &mut __bumps,
                        &mut __reallocs,
                    )?;

                    // Invoke user defined handler.
                    let result = #program_name::#ix_method_name(
                        anchor_lang::context::Context::new(
                            __program_id,
                            &mut __accounts,
                            __remaining_accounts,
                            __bumps,
                        ),
                        #(#ix_arg_names),*
                    )?;

                    // Maybe set Solana return data.
                    #maybe_set_return_data

                    // Exit routine.
                    __accounts.exit(__program_id)
                }
            }
        })
        .collect();

    quote! {
        /// Create a private module to not clutter the program's namespace.
        /// Defines an entrypoint for each individual instruction handler
        /// wrapper.
        mod __private {
            use super::*;

            /// __global mod defines wrapped handlers for global instructions.
            pub mod __global {
                use super::*;

                #(#non_inlined_handlers)*
            }

            #event_cpi_mod
        }
    }
}

/// Generate the event module based on whether the `event-cpi` feature is enabled.
fn generate_event_cpi_mod() -> proc_macro2::TokenStream {
    #[cfg(feature = "event-cpi")]
    {
        let authority = crate::parser::accounts::event_cpi::EventAuthority::get();
        let authority_name = authority.name;

        quote! {
            /// __events mod defines handler for self-cpi based event logging
            pub mod __events {
                use super::*;

                #[inline(never)]
                pub fn __event_dispatch(
                    program_id: &Pubkey,
                    accounts: &[AccountInfo],
                    event_data: &[u8],
                ) -> anchor_lang::Result<()> {
                    let given_event_authority = next_account_info(&mut accounts.iter())?;
                    if !given_event_authority.is_signer {
                        return Err(anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintSigner,
                        )
                        .with_account_name(#authority_name));
                    }

                    if given_event_authority.key() != crate::EVENT_AUTHORITY_AND_BUMP.0 {
                        return Err(anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintSeeds,
                        )
                        .with_account_name(#authority_name)
                        .with_pubkeys((given_event_authority.key(), crate::EVENT_AUTHORITY_AND_BUMP.0)));
                    }

                    Ok(())
                }
            }
        }
    }
    #[cfg(not(feature = "event-cpi"))]
    quote! {}
}
