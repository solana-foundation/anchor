use crate::codegen::program::common::*;
use crate::program_codegen::idl::idl_accounts_and_functions;
use crate::Program;
use quote::{quote, ToTokens};

// Generate non-inlined wrappers for each instruction handler, since Solana's
// BPF max stack size can't handle reasonable sized dispatch trees without doing
// so.
pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    let program_name = &program.name;
    // A constant token stream that stores the accounts and functions, required to live
    // inside the target program in order to get the program ID.
    let idl_accounts_and_functions = idl_accounts_and_functions();
    let non_inlined_idl: proc_macro2::TokenStream = {
        quote! {
            // Entry for all IDL related instructions. Use the "no-idl" feature
            // to eliminate this code, for example, if one wants to make the
            // IDL no longer mutable or if one doesn't want to store the IDL
            // on chain.
            #[inline(never)]
            #[cfg(not(feature = "no-idl"))]
            pub fn __idl_dispatch<'info>(program_id: &Pubkey, accounts: &'info [AccountInfo<'info>], idl_ix_data: &[u8]) -> anchor_lang::Result<()> {
                let mut accounts = accounts;
                let mut data: &[u8] = idl_ix_data;

                let ix = anchor_lang::idl::IdlInstruction::deserialize(&mut data)
                    .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;

                match ix {
                    anchor_lang::idl::IdlInstruction::Create { data_len } => {
                        let mut bumps = <IdlCreateAccounts as anchor_lang::Bumps>::Bumps::default();
                        let mut reallocs = std::collections::BTreeSet::new();
                        let mut accounts =
                            IdlCreateAccounts::try_accounts(program_id, &mut accounts, &[], &mut bumps, &mut reallocs)?;
                        __idl_create_account(program_id, &mut accounts, data_len)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::Resize { data_len } => {
                        let mut bumps = <IdlResizeAccount as anchor_lang::Bumps>::Bumps::default();
                        let mut reallocs = std::collections::BTreeSet::new();
                        let mut accounts =
                            IdlResizeAccount::try_accounts(program_id, &mut accounts, &[], &mut bumps, &mut reallocs)?;
                        __idl_resize_account(program_id, &mut accounts, data_len)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::Close => {
                        let mut bumps = <IdlCloseAccount as anchor_lang::Bumps>::Bumps::default();
                        let mut reallocs = std::collections::BTreeSet::new();
                        let mut accounts =
                            IdlCloseAccount::try_accounts(program_id, &mut accounts, &[], &mut bumps, &mut reallocs)?;
                        __idl_close_account(program_id, &mut accounts)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::CreateBuffer => {
                        let mut bumps = <IdlCreateBuffer as anchor_lang::Bumps>::Bumps::default();
                        let mut reallocs = std::collections::BTreeSet::new();
                        let mut accounts =
                            IdlCreateBuffer::try_accounts(program_id, &mut accounts, &[], &mut bumps, &mut reallocs)?;
                        __idl_create_buffer(program_id, &mut accounts)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::Write { data } => {
                        let mut bumps = <IdlAccounts as anchor_lang::Bumps>::Bumps::default();
                        let mut reallocs = std::collections::BTreeSet::new();
                        let mut accounts =
                            IdlAccounts::try_accounts(program_id, &mut accounts, &[], &mut bumps, &mut reallocs)?;
                        __idl_write(program_id, &mut accounts, data)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::SetAuthority { new_authority } => {
                        let mut bumps = <IdlAccounts as anchor_lang::Bumps>::Bumps::default();
                        let mut reallocs = std::collections::BTreeSet::new();
                        let mut accounts =
                            IdlAccounts::try_accounts(program_id, &mut accounts, &[], &mut bumps, &mut reallocs)?;
                        __idl_set_authority(program_id, &mut accounts, new_authority)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::SetBuffer => {
                        let mut bumps = <IdlSetBuffer as anchor_lang::Bumps>::Bumps::default();
                        let mut reallocs = std::collections::BTreeSet::new();
                        let mut accounts =
                            IdlSetBuffer::try_accounts(program_id, &mut accounts, &[], &mut bumps, &mut reallocs)?;
                        __idl_set_buffer(program_id, &mut accounts)?;
                        accounts.exit(program_id)?;
                    },
                }
                Ok(())
            }

        }
    };

    let event_cpi_mod = generate_event_cpi_mod();

    let non_inlined_handlers: Vec<proc_macro2::TokenStream> = program
        .ixs
        .iter()
        .map(|ix| {
            let ix_arg_names: Vec<&syn::Ident> = ix.args.iter().map(|arg| &arg.name).collect();
            let ix_method_name = &ix.raw_method.sig.ident;
            let ix_method_name_str = ix_method_name.to_string();
            let ix_name = generate_ix_variant_name(&ix_method_name_str);
            let variant_arm = generate_ix_variant(&ix_method_name_str, &ix.args);
            let ix_name_log = format!("Instruction: {ix_name}");
            let anchor = &ix.anchor_ident;
            let ret_type = &ix.returns.ty.to_token_stream();
            let cfgs = &ix.cfgs;
            let maybe_set_return_data = match ret_type.to_string().as_str() {
                "()" => quote! {},
                _ => quote! {
                    let mut return_data = Vec::with_capacity(256);
                    result.serialize(&mut return_data).unwrap();
                    anchor_lang::solana_program::program::set_return_data(&return_data);
                },
            };

            let actual_param_count = ix.args.len();
            let ix_name_str = ix_method_name.to_string();
            let accounts_type_str = anchor.to_string();

            // Build clear error messages
            let count_error_msg = format!(
                "#[instruction(...)] on Account `{}<'_>` expects MORE args, the ix `{}(...)` has only {} args.",
                accounts_type_str,
                ix_name_str,
                actual_param_count,
            );

            // Generate type validation calls for each argument
            let type_validations: Vec<proc_macro2::TokenStream> = ix.args
                .iter()
                .enumerate()
                .map(|(idx, arg)| {
                    let arg_ty = &arg.raw_arg.ty;
                    let method_name = syn::Ident::new(
                        &format!("__anchor_validate_ix_arg_type_{}", idx),
                        proc_macro2::Span::call_site(),
                    );
                    quote! {
                        // Type validation for argument #idx
                        if #anchor::__ANCHOR_IX_PARAM_COUNT > #idx {
                            #[allow(unreachable_code)]
                            if false {
                                // This code is never executed but is type-checked at compile time
                                let __type_check_arg: #arg_ty = panic!();
                                #anchor::#method_name(&__type_check_arg);
                            }
                        }
                    }
                })
                .collect();

            let param_validation = quote! {
                const _: () = {
                    const EXPECTED_COUNT: usize = #anchor::__ANCHOR_IX_PARAM_COUNT;
                    const HANDLER_PARAM_COUNT: usize = #actual_param_count;

                    // Count validation
                    if EXPECTED_COUNT > HANDLER_PARAM_COUNT {
                        panic!(#count_error_msg);
                    }
                };

                // Type validations
                #(#type_validations)*
            };

            quote! {
                #(#cfgs)*
                #[inline(never)]
                pub fn #ix_method_name<'info>(
                    __program_id: &Pubkey,
                    __accounts: &'info[AccountInfo<'info>],
                    __ix_data: &[u8],
                ) -> anchor_lang::Result<()> {
                    #[cfg(not(feature = "no-log-ix-name"))]
                    anchor_lang::prelude::msg!(#ix_name_log);

                    #param_validation
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
            /// __idl mod defines handlers for injected Anchor IDL instructions.
            pub mod __idl {
                use super::*;

                #non_inlined_idl
                #idl_accounts_and_functions
            }

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
