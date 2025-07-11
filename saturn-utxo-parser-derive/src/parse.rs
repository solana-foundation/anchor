#![allow(dead_code)]
//! Parsing routines converting `syn::DeriveInput` into crate-internal IR.

use proc_macro2::Span;
use quote::ToTokens;
use syn::{spanned::Spanned, Expr, Lit};

use crate::ir::{DeriveInputIr, Field, FieldKind, RunesPresence, UtxoAttr};

/// Convert a `syn::DeriveInput` representing the struct annotated with
/// `#[derive(UtxoParser)]` into the crate's internal IR.
///
/// The function purposely performs *syntax*-level extraction only; semantic
/// validation (duplicate anchors, rest‐rules, etc.) is deferred to the
/// `validate` module.
pub fn derive_input_to_ir(input: &syn::DeriveInput) -> syn::Result<DeriveInputIr> {
    // ---------------------------------------------------------------------
    // Fetch the `#[utxo_accounts(TypePath)]` attribute.
    // ---------------------------------------------------------------------
    let mut accounts_ty: Option<syn::Type> = None;
    for attr in &input.attrs {
        if attr.path().is_ident("utxo_accounts") {
            if accounts_ty.is_some() {
                return Err(syn::Error::new(
                    attr.span(),
                    "duplicate #[utxo_accounts] attribute",
                ));
            }
            accounts_ty = Some(attr.parse_args::<syn::Type>()?);
        }
    }

    let accounts_ty = accounts_ty.ok_or_else(|| {
        syn::Error::new(
            input.ident.span(),
            "missing required #[utxo_accounts(<Type>)] attribute",
        )
    })?;

    // ---------------------------------------------------------------------
    // Ensure we are dealing with a struct with named fields.
    // ---------------------------------------------------------------------
    let fields_named = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(named) => &named.named,
            other => {
                return Err(syn::Error::new(
                    other.span(),
                    "UtxoParser only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new(
                input.span(),
                "UtxoParser can only be derived for structs",
            ));
        }
    };

    // ---------------------------------------------------------------------
    // Walk through fields.
    // ---------------------------------------------------------------------
    let mut fields_ir = Vec::<Field>::new();

    for field in fields_named {
        let ident = field.ident.clone().expect("named field");
        let span = field.span();

        // --------------------------------------------------------------
        // Parse #[utxo(..)] attribute if present.
        // --------------------------------------------------------------
        let mut attr = UtxoAttr::default();
        // Track whether we have already seen a `#[utxo(..)]` attribute on this field so we
        // can emit an error if the user attaches more than one.  Historically we silently
        // merged multiple attributes which could hide mistakes – explicit rejection is
        // clearer and keeps the macro input surface small.
        let mut utxo_attr_seen = false;

        for attr_syn in &field.attrs {
            if !attr_syn.path().is_ident("utxo") {
                continue;
            }

            // Reject a second `#[utxo(..)]` attribute on the *same* field.
            if utxo_attr_seen {
                return Err(syn::Error::new(
                    attr_syn.span(),
                    "duplicate #[utxo(...)] attribute on the same field; combine all options into a single attribute",
                ));
            }
            utxo_attr_seen = true;
            attr.span = attr_syn.span();
            let args = attr_syn.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            )?;

            for meta in args {
                match meta {
                    syn::Meta::Path(p) => {
                        if p.is_ident("rest") {
                            if attr.rest {
                                return Err(syn::Error::new(
                                    p.span(),
                                    "duplicate `rest` flag in #[utxo] attribute",
                                ));
                            }
                            attr.rest = true;
                        } else {
                            return Err(syn::Error::new(
                                p.span(),
                                "Unknown flag inside #[utxo(...)] attribute",
                            ));
                        }
                    }
                    syn::Meta::NameValue(nv) => {
                        let key =
                            nv.path
                                .get_ident()
                                .map(|id| id.to_string())
                                .ok_or_else(|| {
                                    syn::Error::new(nv.path.span(), "Expected identifier key")
                                })?;
                        match key.as_str() {
                            "value" => {
                                if attr.value.is_some() {
                                    return Err(syn::Error::new(
                                        nv.path.span(),
                                        "duplicate `value` key inside #[utxo(...)] attribute",
                                    ));
                                }
                                // Accept any Rust expression; defer type checking to the compiler.
                                attr.value = Some(nv.value.clone());
                            }
                            "runes" => {
                                if attr.runes.is_some() {
                                    return Err(syn::Error::new(
                                        nv.path.span(),
                                        "duplicate `runes` key inside #[utxo(...)] attribute",
                                    ));
                                }
                                if let Expr::Lit(expr_lit) = &nv.value {
                                    if let Lit::Str(lit_str) = &expr_lit.lit {
                                        attr.runes = match lit_str.value().as_str() {
                                            "none" => Some(RunesPresence::None),
                                            "some" => Some(RunesPresence::Some),
                                            "any" => Some(RunesPresence::Any),
                                            other => {
                                                return Err(syn::Error::new(
                                                    lit_str.span(),
                                                    format!(
                                                        "unsupported runes value '{}'. expected 'none', 'some', or 'any'",
                                                        other
                                                    ),
                                                ));
                                            }
                                        };
                                    } else {
                                        return Err(syn::Error::new(
                                            expr_lit.span(),
                                            "`runes` expects a string literal",
                                        ));
                                    }
                                } else {
                                    return Err(syn::Error::new(
                                        nv.value.span(),
                                        "`runes` expects a string literal",
                                    ));
                                }
                            }
                            "rune_id" => {
                                if attr.rune_id_expr.is_some() {
                                    return Err(syn::Error::new(
                                        nv.path.span(),
                                        "duplicate `rune_id` key inside #[utxo(...)] attribute",
                                    ));
                                }
                                // Store the expression verbatim – it can be any valid Rust expr path/value.
                                attr.rune_id_expr = Some(nv.value.clone());
                            }
                            "rune_amount" => {
                                if attr.rune_amount_expr.is_some() {
                                    return Err(syn::Error::new(
                                        nv.path.span(),
                                        "duplicate `rune_amount` key inside #[utxo(...)] attribute",
                                    ));
                                }
                                attr.rune_amount_expr = Some(nv.value.clone());
                            }
                            "anchor" => {
                                if attr.anchor_ident.is_some() {
                                    return Err(syn::Error::new(
                                        nv.path.span(),
                                        "duplicate `anchor` key inside #[utxo(...)] attribute",
                                    ));
                                }
                                if let Expr::Path(expr_path) = &nv.value {
                                    if let Some(id) = expr_path.path.get_ident() {
                                        attr.anchor_ident = Some(id.clone());
                                    } else {
                                        return Err(syn::Error::new(
                                            expr_path.span(),
                                            "anchor expects an identifier",
                                        ));
                                    }
                                } else {
                                    return Err(syn::Error::new(
                                        nv.value.span(),
                                        "anchor expects an identifier path",
                                    ));
                                }
                            }
                            other => {
                                return Err(syn::Error::new(
                                    nv.path.span(),
                                    format!("Unknown key '{}' in #[utxo(...)] attribute", other),
                                ));
                            }
                        }
                    }
                    _ => {
                        return Err(syn::Error::new(
                            meta.span(),
                            "Unsupported meta in attribute",
                        ));
                    }
                }
            }
        }

        // --------------------------------------------------------------
        // Deduce FieldKind from `ty`.
        // --------------------------------------------------------------
        let kind = match &field.ty {
            syn::Type::Reference(_) => {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "&UtxoInfo references are no longer supported; use owned `UtxoInfo` values instead",
                ));
            }
            syn::Type::Array(arr) => match &arr.len {
                Expr::Lit(expr_lit) => {
                    ensure_utxo_info_type(&*arr.elem)?;
                    if let Lit::Int(lit_int) = &expr_lit.lit {
                        let len = lit_int.base10_parse::<usize>()?;
                        FieldKind::Array(len)
                    } else {
                        return Err(syn::Error::new(
                            expr_lit.span(),
                            "array length must be an integer literal",
                        ));
                    }
                }
                other => {
                    return Err(syn::Error::new(
                        other.span(),
                        "array length must be an integer literal",
                    ));
                }
            },
            syn::Type::Path(type_path) => {
                if let Some(seg) = type_path.path.segments.last() {
                    match seg.ident.to_string().as_str() {
                        "Vec" => {
                            // Ensure the generic parameter is `UtxoInfo` (or a path ending with it).
                            validate_utxo_info_generic(seg, type_path)?;
                            FieldKind::Vec
                        }
                        "Option" => {
                            validate_utxo_info_generic(seg, type_path)?;
                            FieldKind::Optional
                        }
                        // Bare `UtxoInfo` (without reference) is now allowed as a single owned field.
                        "UtxoInfo" => FieldKind::Single,
                        _ => {
                            return Err(syn::Error::new(
                                type_path.span(),
                                "Unsupported field type for UtxoParser derive. Expected Vec<UtxoInfo>, Option<UtxoInfo>, UtxoInfo, or array [UtxoInfo; N]",
                            ));
                        }
                    }
                } else {
                    return Err(syn::Error::new(type_path.span(), "Unexpected type path"));
                }
            }
            other => {
                return Err(syn::Error::new(
                    other.span(),
                    "Unsupported field type for UtxoParser derive",
                ));
            }
        };

        fields_ir.push(Field {
            ident,
            kind,
            ty: field.ty.clone(),
            attr,
            span,
        });
    }

    Ok(DeriveInputIr {
        struct_ident: input.ident.clone(),
        generics: input.generics.clone(),
        accounts_ty,
        fields: fields_ir,
    })
}

fn expr_to_string(expr: &Expr) -> String {
    expr.to_token_stream().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::RunesPresence;
    use quote::ToTokens;

    fn parse_di(src: &str) -> syn::DeriveInput {
        syn::parse_str::<syn::DeriveInput>(src).expect("unable to parse code")
    }

    #[test]
    fn parses_basic_struct() {
        let code = r#"
            #[utxo_accounts(DummyAccs)]
            struct Simple {
                #[utxo(value = 1_000, runes = "none")]
                fee: saturn_bitcoin_transactions::utxo_info::UtxoInfo,
            }
        "#;
        let di = parse_di(code);
        let ir = derive_input_to_ir(&di).expect("parse ok");
        assert_eq!(ir.fields.len(), 1);
        let f = &ir.fields[0];
        // Ensure the value expression was captured (string match is sufficient).
        let value_str = f
            .attr
            .value
            .as_ref()
            .map(|e| e.to_token_stream().to_string())
            .unwrap();
        let normalized = value_str.replace([' ', '_'], "");
        assert_eq!(normalized, "1000");
        assert_eq!(f.attr.runes, Some(RunesPresence::None));
    }
}

// Helper: verify that the last segment's generic argument is exactly `UtxoInfo` (by ident), otherwise return an error.
fn validate_utxo_info_generic(
    seg: &syn::PathSegment,
    type_path: &syn::TypePath,
) -> syn::Result<()> {
    use syn::{GenericArgument, PathArguments, Type};

    let err = || {
        syn::Error::new(
            type_path.span(),
            "Expected Vec<UtxoInfo> / Option<UtxoInfo> for UtxoParser derive",
        )
    };

    match &seg.arguments {
        PathArguments::AngleBracketed(ab) => {
            if ab.args.len() != 1 {
                return Err(err());
            }
            if let Some(GenericArgument::Type(inner_ty)) = ab.args.first() {
                match inner_ty {
                    Type::Path(inner_path) => {
                        if let Some(last) = inner_path.path.segments.last() {
                            if last.ident == "UtxoInfo" {
                                return Ok(());
                            }
                        }
                        Err(err())
                    }
                    _ => Err(err()),
                }
            } else {
                Err(err())
            }
        }
        _ => Err(err()),
    }
}

// NEW: helper for validating that an arbitrary `Type` is (or ends with) `UtxoInfo`.
fn ensure_utxo_info_type(ty: &syn::Type) -> syn::Result<()> {
    use syn::Type;

    let is_utxo = match ty {
        Type::Path(inner_path) => inner_path
            .path
            .segments
            .last()
            .map(|s| s.ident == "UtxoInfo")
            .unwrap_or(false),
        _ => false,
    };

    if is_utxo {
        Ok(())
    } else {
        Err(syn::Error::new(
            ty.span(),
            "Array element type must be `UtxoInfo` for UtxoParser derive",
        ))
    }
}
