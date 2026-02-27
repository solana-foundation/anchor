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
            let accounts_type_str = anchor.to_string();

            let handler_arg_names: Vec<String> = ix.args.iter().map(|arg| arg.name.to_string()).collect();
            let handler_arg_names_normalized: Vec<String> = handler_arg_names
                .iter()
                .map(|n| n.strip_prefix('_').unwrap_or(n).to_string())
                .collect();
            let handler_arg_names_lit: Vec<proc_macro2::TokenStream> = handler_arg_names_normalized
                .iter()
                .map(|name| quote! { #name })
                .collect();

            // Resolve #[instruction(...)] args once — shared by skip code, type, name, and order checks
            let instruction_args: Option<Vec<(String, Box<syn::Type>)>> = {
                let extract = |s: &syn::ItemStruct| -> Option<Vec<(String, Box<syn::Type>)>> {
                    if s.ident != *anchor { return None; }
                    accounts_parser::parse(s).ok()
                        .and_then(|accs| accs.instruction_api.as_ref().map(|ix_api| {
                            ix_api.iter().filter_map(|expr| {
                                if let syn::Expr::Type(et) = expr {
                                    use crate::parser;
                                    Some((parser::tts_to_string(&et.expr).trim().to_string(), et.ty.clone()))
                                } else { None }
                            }).collect()
                        }))
                };
                let find_in = |items: &[syn::Item]| {
                    items.iter().find_map(|i| match i {
                        syn::Item::Struct(ref s) => extract(s),
                        _ => None,
                    })
                };
                let parse_and_find = |content: String| {
                    syn::parse_file(&content).ok().and_then(|f| find_in(&f.items))
                };

                program.program_mod.content.as_ref()
                    .and_then(|(_, items)| find_in(items))
                    .or_else(|| {
                        let file_path = anchor.span().local_file()
                            .or_else(|| program.program_mod.ident.span().local_file());
                        if let Some(path) = file_path {
                            return std::fs::read_to_string(&path).ok().and_then(parse_and_find);
                        }
                        let cwd = std::env::current_dir().ok()?;
                        let mut paths = vec![cwd.join("src").join("lib.rs")];
                        if let Ok(dir) = std::fs::read_dir(cwd.join("programs")) {
                            for entry in dir.flatten() {
                                if entry.file_type().ok().map(|t| t.is_dir()).unwrap_or(false) {
                                    let p = entry.path().join("src").join("lib.rs");
                                    if p.exists() { paths.push(p); }
                                }
                            }
                        }
                        paths.into_iter()
                            .find_map(|p| if p.exists() { std::fs::read_to_string(&p).ok() } else { None })
                            .and_then(parse_and_find)
                    })
            };

            // Determine mode: if the first #[instruction] arg name matches a handler arg,
            // use name-based matching (skip logic + validation). Otherwise fall back to
            // positional model (no skipping, no name/order checks).
            let name_mode = if let Some(ref ix_args) = instruction_args {
                if let Some((first_name, _)) = ix_args.first() {
                    let first_norm = first_name.strip_prefix('_').unwrap_or(first_name);
                    handler_arg_names_normalized.iter().any(|n| n == first_norm)
                } else {
                    false
                }
            } else {
                false
            };

            // Generate skip code that handles both contiguous and non-contiguous #[instruction] args.
            let (skip_code, use_skipped_data) = if let Some(ref ix_args) = instruction_args {
                if ix_args.is_empty() {
                    (
                        quote! { let __ix_data_for_accounts: &[u8] = &[]; },
                        quote! { __ix_data_for_accounts },
                    )
                } else if !name_mode {
                    // Positional mode: names don't match handler args, pass data as-is
                    (
                        quote! { let __ix_data_for_accounts = __ix_data; },
                        quote! { __ix_data_for_accounts },
                    )
                } else {
                    use std::collections::HashSet;
                    let ix_name_set: HashSet<String> = ix_args.iter()
                        .map(|(name, _)| name.strip_prefix('_').unwrap_or(name).to_string())
                        .collect();

                    // Map instruction arg names to their positions in the handler signature
                    let ix_positions: Vec<usize> = handler_arg_names_normalized.iter().enumerate()
                        .filter(|(_, name)| ix_name_set.contains(name.as_str()))
                        .map(|(i, _)| i)
                        .collect();

                    if ix_positions.is_empty() {
                        // Names didn't match — compile-time name check will catch this
                        (
                            quote! { let __ix_data_for_accounts = __ix_data; },
                            quote! { __ix_data_for_accounts },
                        )
                    } else {
                        let first_pos = ix_positions[0];
                        let is_contiguous = ix_positions.len() == 1
                            || ix_positions.windows(2).all(|w| w[1] == w[0] + 1);

                        if is_contiguous {
                            // Simple prefix-skip: deserialize-and-discard handler args before first instruction arg
                            let prefix_skips: Vec<proc_macro2::TokenStream> = ix.args.iter()
                                .take(first_pos)
                                .map(|arg| {
                                    let arg_ty = &arg.raw_arg.ty;
                                    quote! {
                                        let _: #arg_ty = anchor_lang::AnchorDeserialize::deserialize(&mut __ix_data_for_accounts)
                                            .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
                                    }
                                })
                                .collect();

                            (
                                quote! {
                                    let mut __ix_data_for_accounts = __ix_data;
                                    #(#prefix_skips)*
                                },
                                quote! { __ix_data_for_accounts },
                            )
                        } else {
                            // Non-contiguous: selectively copy instruction arg bytes into a Vec,
                            // skipping intermediate handler args that aren't in #[instruction].
                            let last_pos = *ix_positions.last().unwrap();
                            let selective_ops: Vec<proc_macro2::TokenStream> = ix.args.iter()
                                .take(last_pos + 1)
                                .enumerate()
                                .map(|(idx, arg)| {
                                    let arg_ty = &arg.raw_arg.ty;
                                    if ix_name_set.contains(handler_arg_names_normalized[idx].as_str()) {
                                        // Instruction arg — record byte span and copy to output
                                        quote! {
                                            {
                                                let __start = __ix_data.len() - __ix_data_cursor.len();
                                                let _: #arg_ty = anchor_lang::AnchorDeserialize::deserialize(&mut __ix_data_cursor)
                                                    .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
                                                let __end = __ix_data.len() - __ix_data_cursor.len();
                                                __ix_data_for_accounts.extend_from_slice(&__ix_data[__start..__end]);
                                            }
                                        }
                                    } else {
                                        // Not an instruction arg — deserialize and discard to advance cursor
                                        quote! {
                                            {
                                                let _: #arg_ty = anchor_lang::AnchorDeserialize::deserialize(&mut __ix_data_cursor)
                                                    .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
                                            }
                                        }
                                    }
                                })
                                .collect();

                            (
                                quote! {
                                    let mut __ix_data_cursor = &__ix_data[..];
                                    let mut __ix_data_for_accounts: Vec<u8> = Vec::new();
                                    #(#selective_ops)*
                                },
                                quote! { &__ix_data_for_accounts },
                            )
                        }
                    }
                }
            } else {
                // Fallback: runtime prefix-skip (when instruction_args couldn't be resolved at macro time)
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

                (
                    quote! {
                        let mut __ix_data_for_accounts = __ix_data;
                        const HANDLER_ARG_NAMES: &[&str] = &[#(#handler_arg_names_lit),*];
                        let ix_arg_names = #anchor::__anchor_ix_arg_names();

                        if !ix_arg_names.is_empty() && !HANDLER_ARG_NAMES.is_empty() {
                            let __first_ix = ix_arg_names[0].strip_prefix('_').unwrap_or(ix_arg_names[0]);
                            let mut first_match_idx = None;
                            for (handler_idx, handler_name) in HANDLER_ARG_NAMES.iter().enumerate() {
                                if *handler_name == __first_ix {
                                    first_match_idx = Some(handler_idx);
                                    break;
                                }
                            }

                            if let Some(skip_count) = first_match_idx {
                                #(#skip_deserializations)*
                            }
                        }
                    },
                    quote! { __ix_data_for_accounts },
                )
            };

            // Build clear error messages
            let count_error_msg = format!(
                "{}'s expects more args than ix function `{}` provides.",
                accounts_type_str,
                ix_method_name_str
            );

            // Type validation
            let type_validations: Vec<proc_macro2::TokenStream> = {
                use std::collections::HashMap;
                let handler_args_map: HashMap<String, &syn::Type> = ix.args.iter()
                    .map(|arg| (arg.name.to_string(), &*arg.raw_arg.ty))
                    .collect();

                if name_mode {
                    let ix_args = instruction_args.as_ref().unwrap();
                    ix_args.iter().enumerate().filter_map(|(idx, (name, _))| {
                        let method = syn::Ident::new(
                            &format!("__anchor_validate_ix_arg_type_{}", idx),
                            proc_macro2::Span::call_site(),
                        );
                        handler_args_map.get(name).map(|ty| quote! {
                            #[allow(unreachable_code)]
                            if false {
                                let __type_check_arg: #ty = panic!();
                                #anchor::#method(&__type_check_arg);
                            }
                        })
                    }).collect()
                } else {
                    (0..ix.args.len().min(32)).map(|idx| {
                        let ty = &*ix.args[idx].raw_arg.ty;
                        let method = syn::Ident::new(
                            &format!("__anchor_validate_ix_arg_type_{}", idx),
                            proc_macro2::Span::call_site(),
                        );
                        quote! {
                            #[allow(unreachable_code)]
                            if false {
                                let __type_check_arg: #ty = panic!();
                                #anchor::#method(&__type_check_arg);
                            }
                        }
                    }).collect()
                }
            };

            // Name + order validation (only in name-based mode)
            let name_and_order_checks: Vec<proc_macro2::TokenStream> = {
                if name_mode {
                    let ix_args = instruction_args.as_ref().unwrap();
                    use std::collections::HashMap;
                    let handler_set: std::collections::HashSet<&str> = handler_arg_names_normalized.iter()
                        .map(|s| s.as_str()).collect();
                    let handler_pos: HashMap<&str, usize> = handler_arg_names_normalized.iter()
                        .enumerate().map(|(i, n)| (n.as_str(), i)).collect();

                    let mut checks = Vec::new();

                    // Every #[instruction] arg must exist in handler
                    for (idx, (name, _)) in ix_args.iter().enumerate() {
                        let norm = name.strip_prefix('_').unwrap_or(name);
                        if !handler_set.contains(norm) {
                            let msg = format!(
                                "{}'s ix arg '{}' at index {} is not found in {}.",
                                accounts_type_str, name, idx, ix_method_name_str
                            );
                            checks.push(quote! { const _: () = { panic!(#msg); }; });
                        }
                    }

                    // Positions in handler must be strictly increasing
                    let named_positions: Vec<(&str, usize)> = ix_args.iter()
                        .filter_map(|(name, _)| {
                            let norm = name.strip_prefix('_').unwrap_or(name);
                            handler_pos.get(norm).map(|&pos| (norm, pos))
                        })
                        .collect();
                    for w in named_positions.windows(2) {
                        if w[0].1 >= w[1].1 {
                            let msg = format!(
                                "{}'s ix arg '{}' is not found after '{}'.",
                                accounts_type_str, w[1].0, w[0].0
                            );
                            checks.push(quote! { const _: () = { panic!(#msg); }; });
                        }
                    }
                    checks
                } else {
                    vec![]
                }
            };

            let param_validation = quote! {
                const _: () = {
                    const EXPECTED_COUNT: usize = #anchor::__ANCHOR_IX_PARAM_COUNT;
                    const HANDLER_PARAM_COUNT: usize = #actual_param_count;
                    if EXPECTED_COUNT > HANDLER_PARAM_COUNT {
                        panic!(#count_error_msg);
                    }
                };
                #(#name_and_order_checks)*
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

                    // Deserialize accounts
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
