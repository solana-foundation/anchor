//! This file now contains **full** generator routines that emit exactly the same
//! extraction semantics that the original `derive_utxo_parser_old` macro
//! provided, but working from the crate-internal IR.  The implementation is
//! intentionally verbose so that the generated source mirrors the proven logic
//! one-to-one.

use crate::parser::utxo::ir::{Field, FieldKind};
use quote::quote;

/// Helper: choose the `ErrorCode` variant that should be used when the field
/// fails to match **without** needing the specialised RuneId/RuneAmount logic.
fn base_error_variant(attr: &crate::parser::utxo::ir::UtxoAttr) -> proc_macro2::TokenStream {
    // Anchored fields implicitly require `runes == none` even if the user did
    // not specify the `runes` flag.  Therefore their failure mode should be
    // `InvalidRunesPresence` when the predicate does not match.
    if attr.anchor_ident.is_some() && attr.runes.is_none() {
        return quote! { ErrorCode::InvalidRunesPresence };
    }
    if attr.rune_id_expr.is_some() {
        quote! { ErrorCode::InvalidRuneId }
    } else if attr.rune_amount_expr.is_some() {
        quote! { ErrorCode::InvalidRuneAmount }
    } else if attr.runes.is_some() {
        quote! { ErrorCode::InvalidRunesPresence }
    } else if attr.value.is_some() {
        quote! { ErrorCode::InvalidUtxoValue }
    } else {
        quote! { ErrorCode::MissingRequiredUtxo }
    }
}

/// Build the `TokenStream` that initialises the given field using a variable
/// named `remaining` (`Vec<UtxoInfo>`) and assuming a variable `accounts` in
/// scope.  `predicate` **must** be an expression that can be evaluated for a
/// `utxo` identifier.
pub fn build_extractor(
    field: &Field,
    predicate: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let ident = &field.ident;
    let attr = &field.attr;
    // Pre-compute the specific error variant for predicate mismatch.
    let err_variant = base_error_variant(attr);

    match field.kind {
        // ------------------------------------------------------------------
        // Single `UtxoInfo`
        // ------------------------------------------------------------------
        FieldKind::Single => {
            let anchor_snippet = if let Some(anchor_ident) = &attr.anchor_ident {
                let anchor_ident_tok = anchor_ident.clone();
                quote! {
                    // Compile-time assertion: the anchor target for a scalar UTXO field **must** itself
                    // be "scalar‐like", i.e. directly convertible via `ToAccountInfo`.  This produces a
                    // clear error message such as:
                    //     "the trait `ToAccountInfo` is not implemented for `Vec<...>`"
                    // when the developer accidentally anchors to a `Vec` inside the `Accounts` struct.
                    {
                        fn _anchor_scalar_check<'info, T>(_: &T)
                        where
                            T: anchor_lang::ToAccountInfo<'info>,
                        {
                        }
                        _anchor_scalar_check(&ctx.accounts.#anchor_ident_tok);
                    }
                    let _anchor_target = anchor_lang::ToAccountInfo::to_account_info(&ctx.accounts.#anchor_ident_tok);
                    let _anchor_ix = arch_program::system_instruction::anchor(
                        _anchor_target.key,
                        #ident.meta.txid_big_endian(),
                        #ident.meta.vout(),
                    );
                    // Record state transition for this anchored account in the Bitcoin
                    // transaction builder.
                    ctx.btc_tx_builder.add_state_transition(&_anchor_target)?;
                }
            } else {
                quote! {}
            };

            // Choose correct error variant if predicate fails.
            let err_on_mismatch = if attr.value.is_none()
                && attr.runes.is_none()
                && attr.rune_id_expr.is_none()
                && attr.rune_amount_expr.is_none()
            {
                // No predicates – only order matters
                quote! { ErrorCode::StrictOrderMismatch }
            } else {
                // Map to the specific predicate-related error
                err_variant.clone()
            };

            // Special handling when both rune_id and rune_amount are specified to distinguish
            // between ID vs amount mismatch at runtime.
            let rune_mismatch_logic = if let (Some(id_expr), Some(_)) =
                (&attr.rune_id_expr, &attr.rune_amount_expr)
            {
                quote! {
                    if !(#predicate) {
                        // Decide whether the ID matched but amount mismatched, or ID mismatched.
                        if utxo.rune_amount(&(#id_expr)).is_some() {
                            return Err(ProgramError::Custom(ErrorCode::InvalidRuneAmount.into()));
                        } else {
                            return Err(ProgramError::Custom(ErrorCode::InvalidRuneId.into()));
                        }
                    }
                }
            } else {
                quote! {
                    if !(#predicate) {
                        return Err(ProgramError::Custom(#err_on_mismatch.into()));
                    }
                }
            };

            quote! {
                if idx >= total {
                    return Err(ProgramError::Custom(ErrorCode::MissingRequiredUtxo.into()));
                }
                let utxo = anchor_lang::utxo_parser::meta_to_info(&utxos[idx])?;
                #rune_mismatch_logic
                let #ident = utxo;
                idx += 1;
                #anchor_snippet
            }
        }
        // ------------------------------------------------------------------
        // Optional UtxoInfo (Option)
        // ------------------------------------------------------------------
        FieldKind::Optional => {
            let anchor_snippet = if let Some(anchor_ident) = &attr.anchor_ident {
                let anchor_ident_tok = anchor_ident.clone();
                quote! {
                    if let Some(__opt_utxo) = #ident.as_ref() {
                        // Compile-time assertion identical to the one for scalar fields – ensure the
                        // anchor target itself is scalar and not a collection.
                        {
                            fn _anchor_scalar_check<'info, T>(_: &T)
                            where
                                T: anchor_lang::ToAccountInfo<'info>,
                            {
                            }
                            _anchor_scalar_check(&ctx.accounts.#anchor_ident_tok);
                        }
                        let _anchor_target = anchor_lang::ToAccountInfo::to_account_info(&ctx.accounts.#anchor_ident_tok);
                        let _anchor_ix = arch_program::system_instruction::anchor(
                            _anchor_target.key,
                            __opt_utxo.meta.txid_big_endian(),
                            __opt_utxo.meta.vout(),
                        );
                        ctx.btc_tx_builder.add_state_transition(&_anchor_target)?;
                    }
                }
            } else {
                quote! {}
            };

            quote! {
                let #ident: Option<satellite_bitcoin::utxo_info::UtxoInfo> = if idx < total {
                    let utxo = anchor_lang::utxo_parser::meta_to_info(&utxos[idx])?;
                    if (#predicate) {
                        idx += 1;
                        Some(utxo)
                    } else {
                        None
                    }
                } else { None };
                #anchor_snippet
            }
        }
        // ------------------------------------------------------------------
        // Fixed-length Array
        // ------------------------------------------------------------------
        FieldKind::Array(len) => {
            let len_lit = len as usize;
            // If this array UTXO field is anchored, perform a compile-time assertion that the
            // chosen accounts field *can* be indexed.  This emits a trait-bound error that points
            // at the macro input rather than a deep generated loop, giving the user a clearer
            // diagnostic ("anchor target `foo` must implement Index<usize>").
            let anchor_preflight = if let Some(anchor_ident) = &attr.anchor_ident {
                let anchor_ident_tok = anchor_ident.clone();
                quote! {
                    // Compile-time assertion helper – never executed at runtime.
                    let _ = {
                        fn _assert_indexable<T: core::ops::Index<usize>>(_t: &T) {}
                        _assert_indexable(&ctx.accounts.#anchor_ident_tok);
                    };
                }
            } else {
                quote! {}
            };
            // Build per-element initialisation blocks.
            let mut element_blocks: Vec<proc_macro2::TokenStream> = Vec::new();
            for i in 0..len_lit {
                let anchor_stmt = if let Some(anchor_ident) = &attr.anchor_ident {
                    let anchor_ident_tok = anchor_ident.clone();
                    quote! {
                        let _anchor_target = anchor_lang::ToAccountInfo::to_account_info(&ctx.accounts.#anchor_ident_tok[#i]);
                        let _anchor_ix = arch_program::system_instruction::anchor(
                            _anchor_target.key,
                            utxo.meta.txid_big_endian(),
                            utxo.meta.vout(),
                        );
                        ctx.btc_tx_builder.add_state_transition(&_anchor_target)?;
                    }
                } else {
                    quote! {}
                };

                element_blocks.push(quote! {
                    {
                        let utxo = anchor_lang::utxo_parser::meta_to_info(&utxos[idx + #i])?;
                        if !(#predicate) {
                            return Err(ProgramError::Custom(#err_variant.into()));
                        }
                        #anchor_stmt
                        utxo
                    }
                });
            }

            // Generate the final extractor snippet.
            quote! {
                #anchor_preflight
                // Ensure enough inputs remain.
                if total < idx + #len_lit {
                    return Err(ProgramError::Custom(ErrorCode::MissingRequiredUtxo.into()));
                }
                let #ident: [satellite_bitcoin::utxo_info::UtxoInfo; #len_lit] = [
                    #( #element_blocks ),*
                ];
                idx += #len_lit;
            }
        }
        // ------------------------------------------------------------------
        // Vec
        // ------------------------------------------------------------------
        FieldKind::Vec => {
            if let Some(anchor_ident) = &attr.anchor_ident {
                let anchor_ident_tok = anchor_ident.clone();
                // Compile-time assertion identical to the Array case – the accounts field must
                // support indexing.
                let anchor_preflight = quote! {
                    let _ = {
                        fn _assert_indexable<T: core::ops::Index<usize>>(_t: &T) {}
                        _assert_indexable(&ctx.accounts.#anchor_ident_tok);
                    };
                };
                quote! {
                    #anchor_preflight
                    let target_len = ctx.accounts.#anchor_ident_tok.len();
                    let mut #ident: Vec<satellite_bitcoin::utxo_info::UtxoInfo> = Vec::with_capacity(target_len);
                    for i in 0..target_len {
                        if idx >= total {
                            return Err(ProgramError::Custom(ErrorCode::MissingRequiredUtxo.into()));
                        }
                        let utxo = anchor_lang::utxo_parser::meta_to_info(&utxos[idx])?;
                        if !(#predicate) {
                            return Err(ProgramError::Custom(#err_variant.into()));
                        }
                        let _anchor_target = anchor_lang::ToAccountInfo::to_account_info(&ctx.accounts.#anchor_ident_tok[i]);
                        let _anchor_ix = arch_program::system_instruction::anchor(
                            _anchor_target.key,
                            utxo.meta.txid_big_endian(),
                            utxo.meta.vout(),
                        );
                        // Record state transition for this anchored account in the Bitcoin
                        // transaction builder (if one is present on the `Context`).
                        ctx.btc_tx_builder.add_state_transition(&_anchor_target)?;
                        #ident.push(utxo);
                        idx += 1;
                    }
                }
            } else if attr.rest {
                // `#[utxo(rest)]` must still flag *unexpected* inputs. We therefore
                // walk over the remaining slice, *collect* those matching the
                // predicate, but advance the main cursor only for the ones we
                // actually consumed. That leaves non-matching inputs in place so
                // the final leftover check can emit `UnexpectedExtraUtxos`.
                quote! {
                    let mut #ident: Vec<satellite_bitcoin::utxo_info::UtxoInfo> = Vec::new();

                    // Remember where the rest segment starts.
                    let start_idx = idx;
                    let mut consumed: usize = 0;

                    for i in start_idx..total {
                        let utxo = anchor_lang::utxo_parser::meta_to_info(&utxos[i])?;
                        if (#predicate) {
                            #ident.push(utxo);
                            consumed += 1;
                        }
                    }

                    // Mark only the captured UTXOs as consumed; any others remain
                    // un-consumed and will trigger the leftover-inputs check.
                    idx += consumed;
                }
            } else {
                syn::Error::new(field.span, "Vec field must be either `rest` or `anchor`")
                    .to_compile_error()
            }
        }
    }
}
