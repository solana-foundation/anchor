use crate::codegen::program::common::*;
use crate::parser::accounts as accounts_parser;
use crate::Program;
use quote::{quote, ToTokens};

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
            let ix_name = match generate_ix_variant_name(&ix_method_name_str) {
                Ok(name) => quote! { #name },
                Err(e) => {
                    let err = e.to_string();
                    return quote! { compile_error!(concat!("error generating ix variant name: `", #err, "`")) };
                }
            };
            let variant_arm = match generate_ix_variant(&ix_method_name_str, &ix.args) {
                Ok(v) => v,
                Err(e) => {
                    let err = e.to_string();
                    return quote! { compile_error!(concat!("error generating ix variant arm: `", #err, "`")) };
                }
            };

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

            // Match instruction arg names to handler arg names to find skip offset
            // Generate code that matches instruction arg names to handler arg indices at compile time
            let handler_arg_names: Vec<String> = ix.args.iter().map(|arg| arg.name.to_string()).collect();
            let handler_arg_names_lit: Vec<proc_macro2::TokenStream> = handler_arg_names
                .iter()
                .map(|name| quote! { #name })
                .collect();

            // Generate skip code: deserialize and discard handler args that come before
            // the first instruction arg. Match instruction arg names to handler arg names.
            // This only supports skipping from the start and then reading sequentially.
            let (skip_code, use_skipped_data) = {
                // Generate deserialize calls for each handler arg that might need to be skipped
                let skip_deserializations: Vec<proc_macro2::TokenStream> = ix.args
                    .iter()
                    .enumerate()
                    .map(|(idx, arg)| {
                        let arg_ty = &arg.raw_arg.ty;
                        quote! {
                            if skip_count > #idx {
                                let _: #arg_ty = anchor_lang::AnchorDeserialize::deserialize(&mut __ix_data_for_accounts)
                                    .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
                            }
                        }
                    })
                    .collect();

                let skip_code_gen = quote! {
                    let mut __ix_data_for_accounts = __ix_data;
                    // Match instruction arg names to handler arg names to find which args to skip
                    const HANDLER_ARG_NAMES: &[&str] = &[#(#handler_arg_names_lit),*];
                    let ix_arg_names = #anchor::__anchor_ix_arg_names();

                    if !ix_arg_names.is_empty() && !HANDLER_ARG_NAMES.is_empty() {
                        // Find the first handler arg index that matches the first instruction arg
                        let mut first_match_idx = None;
                        for (handler_idx, handler_name) in HANDLER_ARG_NAMES.iter().enumerate() {
                            if handler_name == &ix_arg_names[0] {
                                first_match_idx = Some(handler_idx);
                                break;
                            }
                        }

                        if let Some(skip_count) = first_match_idx {
                            // Deserialize and discard handler args before the first instruction arg
                            #(#skip_deserializations)*
                        }
                    }
                };

                (skip_code_gen, quote! { __ix_data_for_accounts })
            };

            // Build clear error messages
            let count_error_msg = format!(
                "#[instruction(...)] on Account `{}<'_>` expects MORE args, the ix `{}(...)` has only {} args.",
                accounts_type_str,
                ix_name_str,
                actual_param_count,
            );

            // Generate type validation calls using name->type maps
            // Try to find AccountsStruct in program module to match names at code generation time
            let type_validations: Vec<proc_macro2::TokenStream> = {
                use std::collections::HashMap;
                let handler_args_map: HashMap<String, &syn::Type> = ix.args
                    .iter()
                    .map(|arg| (arg.name.to_string(), &*arg.raw_arg.ty))
                    .collect();
                let extract_instruction_args = |item_struct: &syn::ItemStruct| -> Option<Vec<(String, Box<syn::Type>)>> {
                    if item_struct.ident != *anchor {
                        return None;
                    }
                    accounts_parser::parse(item_struct).ok()
                        .and_then(|accs_struct| accs_struct.instruction_api.as_ref().map(|ix_api| {
                            ix_api.iter().filter_map(|expr| {
                                if let syn::Expr::Type(expr_type) = expr {
                                    use crate::parser;
                                    let name = parser::tts_to_string(&expr_type.expr).trim().to_string();
                                    Some((name, expr_type.ty.clone()))
                                } else {
                                    None
                                }
                            }).collect()
                        }))
                };
                let instruction_args_opt: Option<Vec<(String, Box<syn::Type>)>> = {
                    let result = program.program_mod.content.as_ref()
                        .and_then(|(_, items)| {
                            items.iter().find_map(|item| {
                                if let syn::Item::Struct(ref item_struct) = item {
                                    extract_instruction_args(item_struct)
                                } else {
                                    None
                                }
                            })
                        });
                    result.or_else(|| {
                        program.program_mod.ident.span().local_file()
                            .and_then(|path| std::fs::read_to_string(&path).ok())
                            .and_then(|content| syn::parse_file(&content).ok())
                            .and_then(|file| {
                                file.items.iter().find_map(|item| {
                                    if let syn::Item::Struct(ref item_struct) = item {
                                        extract_instruction_args(item_struct)
                                    } else {
                                        None
                                    }
                                })
                            })
                    })
                };
                let mut validations = Vec::new();
                if let Some(ref ix_args) = instruction_args_opt {
                    for (ix_arg_idx, (ix_arg_name, _ix_arg_ty)) in ix_args.iter().enumerate() {
                        let method_name = syn::Ident::new(
                            &format!("__anchor_validate_ix_arg_type_{}", ix_arg_idx),
                            proc_macro2::Span::call_site(),
                        );

                        if let Some(handler_ty) = handler_args_map.get(ix_arg_name) {
                            validations.push(quote! {
                                #[allow(unreachable_code)]
                                if false {
                                    let __type_check_arg: #handler_ty = panic!();
                                    #anchor::#method_name(&__type_check_arg);
                                }
                            });
                        }
                    }
                } else {
                    for handler_idx in 0..ix.args.len().min(32) {
                        let handler_ty = &*ix.args[handler_idx].raw_arg.ty;
                        let method_name = syn::Ident::new(
                            &format!("__anchor_validate_ix_arg_type_{}", handler_idx),
                            proc_macro2::Span::call_site(),
                        );
                        validations.push(quote! {
                            #[allow(unreachable_code)]
                            if false {
                                let __type_check_arg: #handler_ty = panic!();
                                #anchor::#method_name(&__type_check_arg);
                            }
                        });
                    }
                }

                validations
            };

            let name_checks: Vec<proc_macro2::TokenStream> = {
                let handler_arg_names_set: std::collections::HashSet<String> = ix.args
                    .iter()
                    .map(|arg| {
                        let name = arg.name.to_string();
                        name.strip_prefix('_').unwrap_or(&name).to_string()
                    })
                    .collect();
                let extract_names = |item_struct: &syn::ItemStruct| -> Option<Vec<String>> {
                    if item_struct.ident != *anchor {
                        return None;
                    }
                    accounts_parser::parse(item_struct).ok()
                        .and_then(|accs_struct| accs_struct.instruction_api.as_ref().map(|ix_api| {
                            ix_api.iter().filter_map(|expr| {
                                if let syn::Expr::Type(expr_type) = expr {
                                    use crate::parser;
                                    Some(parser::tts_to_string(&expr_type.expr).trim().to_string())
                                } else {
                                    None
                                }
                            }).collect()
                        }))
                };
                let instruction_arg_names_opt: Option<Vec<String>> = {
                    let result = program.program_mod.content.as_ref()
                        .and_then(|(_, items)| {
                            items.iter().find_map(|item| {
                                if let syn::Item::Struct(ref item_struct) = item {
                                    extract_names(item_struct)
                                } else {
                                    None
                                }
                            })
                        });

                    result.or_else(|| {
                        program.program_mod.ident.span().local_file()
                            .and_then(|path| std::fs::read_to_string(&path).ok())
                            .and_then(|content| syn::parse_file(&content).ok())
                            .and_then(|file| {
                                file.items.iter().find_map(|item| {
                                    if let syn::Item::Struct(ref item_struct) = item {
                                        extract_names(item_struct)
                                    } else {
                                        None
                                    }
                                })
                            })
                    })
                };

                if let Some(ref ix_names) = instruction_arg_names_opt {
                    ix_names.iter().enumerate().map(|(idx, ix_name)| {
                        let normalized_ix_name = ix_name.strip_prefix('_').unwrap_or(ix_name);
                        if !handler_arg_names_set.contains(normalized_ix_name) {
                            quote! {
                                const _: () = {
                                    panic!(concat!(
                                        #count_error_msg,
                                        " Instruction arg '", #ix_name, "' at index ", #idx,
                                        " not found in handler args."
                                    ));
                                };
                            }
                        } else {
                            quote! {}
                        }
                    }).collect()
                } else {
                    vec![]
                }
            };

            let param_validation = quote! {
                const _: () = {
                    const EXPECTED_COUNT: usize = #anchor::__ANCHOR_IX_PARAM_COUNT;
                    const HANDLER_PARAM_COUNT: usize = #actual_param_count;

                    // Validation: instruction args count must not exceed handler args count
                    // (allows partial args, but prevents declaring more args than handler has)
                    if EXPECTED_COUNT > HANDLER_PARAM_COUNT {
                        panic!(#count_error_msg);
                    }
                };

                // Name validation: check instruction arg names exist in handler args
                #(#name_checks)*

                // Type validations
                // Note: For partial args optimization, type validation is relaxed
                // to allow #[instruction] to declare subset of handler args
                // Full type checking happens at runtime during deserialization
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
                    #skip_code

                    // Deserialize data.
                    let ix = instruction::#ix_name::deserialize(&mut &__ix_data[..])
                        .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
                    let instruction::#variant_arm = ix;

                    // Bump collector.
                    let mut __bumps = <#anchor as anchor_lang::Bumps>::Bumps::default();

                    let mut __reallocs = std::collections::BTreeSet::new();

                    // Deserialize accounts (using potentially skipped data if #[instruction] is present).
                    let mut __remaining_accounts: &[AccountInfo] = __accounts;
                    let mut __accounts = #anchor::try_accounts(
                        __program_id,
                        &mut __remaining_accounts,
                        #use_skipped_data,
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
