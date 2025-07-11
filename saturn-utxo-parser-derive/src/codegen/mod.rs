pub mod extractors;
pub mod predicate;

use crate::ir::{DeriveInputIr, RunesPresence};
use quote::quote;
use syn::parse_quote;

/// Build the predicate TokenStream for a field, applying the implicit rule
/// that `anchor = ...` implies `runes == none` when the user did not provide a
/// runes constraint.  This preserves legacy semantics without modifying the
/// parsing stage.
fn build_predicate_with_anchor_logic(field: &crate::ir::Field) -> proc_macro2::TokenStream {
    let mut attr = field.attr.clone();
    if attr.anchor_ident.is_some() && attr.runes.is_none() {
        attr.runes = Some(RunesPresence::None);
    }
    crate::codegen::predicate::build(&attr)
}

/// Assemble the final `TokenStream` implementing `TryFromUtxos` for the target
/// struct.  The generated code mirrors the behaviour of the original
/// `derive_utxo_parser_old` implementation while being driven by the new IR /
/// modular design.
pub fn expand(ir: &DeriveInputIr) -> proc_macro2::TokenStream {
    let struct_ident = &ir.struct_ident;
    let accounts_ty = &ir.accounts_ty;

    // Extract the type-level generics (`<T, 'a, ..>` → used as #ty_generics).
    // We need two different generic lists:
    //  * `ty_generics` – the generics used on the *type* (`Struct<T>`)
    //  * `impl_generics` – the generics for the `impl` block which must
    //    include a fresh lifetime `'a` required by the `TryFromUtxos` trait
    //    *in addition* to whatever generics the struct already has.

    // (a) Keep the original type generics untouched.
    let (_, ty_generics, _) = ir.generics.split_for_impl();

    // (b) Build a new `impl_generics` *by cloning* the struct generics and
    //     ensuring all lifetimes required by the `TryFromUtxos` trait are present.
    let mut impl_generics_mut = ir.generics.clone();

    // The trait expects the following lifetimes (in this exact order):
    //   'utxos, 'a, 'b, 'c, 'info
    // Insert any that are missing at the *front* so we preserve the order
    // "'utxos, 'a, 'b, 'c, 'info, <existing generics...>".
    fn ensure_lifetime(generics: &mut syn::Generics, name: &str) {
        let exists = generics.params.iter().any(|param| match param {
            syn::GenericParam::Lifetime(lt) => lt.lifetime.ident == name,
            _ => false,
        });
        if !exists {
            // SAFETY: parse_quote! can take a literal `'ident`.
            let lt: syn::GenericParam = match name {
                "utxos" => parse_quote!('utxos),
                "a" => parse_quote!('a),
                "b" => parse_quote!('b),
                "c" => parse_quote!('c),
                "info" => parse_quote!('info),
                _ => unreachable!(),
            };
            generics.params.insert(0, lt);
        }
    }

    for &lt_name in ["info", "c", "b", "a", "utxos"].iter() {
        ensure_lifetime(&mut impl_generics_mut, lt_name);
    }
    let (impl_generics, _phantom, where_clause) = impl_generics_mut.split_for_impl();

    // ---------------------------------------------------------------
    // Build extraction snippets in declaration order.
    // ---------------------------------------------------------------
    let mut init_snippets: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut field_idents: Vec<&syn::Ident> = Vec::new();

    // ---------------------------------------------------------------
    // Initialise index-based traversal variables and duplicate check.
    // ---------------------------------------------------------------
    init_snippets.push(quote! {
        // Strict-order parsing state
        let mut idx: usize = 0;
        let total: usize = utxos.len();

        // Optional pre-flight duplicate meta detection (cheap O(n^2) because N is small)
        for i in 0..total {
            for j in (i + 1)..total {
                if utxos[i] == utxos[j] {
                    return Err(ProgramError::Custom(ErrorCode::DuplicateUtxoMeta.into()));
                }
            }
        }
    });

    for field in &ir.fields {
        field_idents.push(&field.ident);
        let predicate_ts = build_predicate_with_anchor_logic(field);
        let extractor_ts = crate::codegen::extractors::build_extractor(field, &predicate_ts);
        init_snippets.push(extractor_ts);
    }

    // Check for leftover inputs after all fields have extracted theirs.
    init_snippets.push(quote! {
        if idx < total {
            return Err(ProgramError::Custom(ErrorCode::UnexpectedExtraUtxos.into()));
        }
    });

    // ---------------------------------------------------------------
    // Compose the final impl block.
    // ---------------------------------------------------------------
    quote! {
        impl #impl_generics saturn_utxo_parser::TryFromUtxos<'utxos, 'a, 'b, 'c, 'info, #accounts_ty<'info>> for #struct_ident #ty_generics #where_clause {
            fn try_utxos(
                ctx: &mut anchor_lang::context::BtcContext<'a, 'b, 'c, 'info, #accounts_ty<'info>>, 
                utxos: &'utxos [arch_program::utxo::UtxoMeta],
            ) -> core::result::Result<Self, arch_program::program_error::ProgramError>
            {
                use arch_program::program_error::ProgramError;
                use saturn_utxo_parser::ErrorCode;

                #(#init_snippets)*

                Ok(Self { #(#field_idents),* })
            }
        }
    }
}
