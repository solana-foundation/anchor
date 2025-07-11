#![allow(dead_code)]
//! Semantic checks for the `UtxoParser` IR.

use crate::ir::{DeriveInputIr, FieldKind};
use syn::spanned::Spanned;

/// Perform post-parse validation on the IR. Returns `Ok(())` if everything
/// is semantically sound; otherwise an appropriate `syn::Error`.
///
/// These checks are intentionally kept separate from parsing so that the logic
/// is unit-testable and the error messages remain focused on semantic
/// problems rather than syntax.
pub fn check(ir: &DeriveInputIr) -> syn::Result<()> {
    use proc_macro2::Span;
    use syn::Error;

    // ---------------------------------------------------------------------
    // Detect mixed usage of the same `anchor = ident` across collection and
    // scalar fields.  When the same Accounts field is referenced by both a
    // collection‐typed UTXO field (Array / Vec) *and* a scalar one
    // (Single / Optional) the generated code becomes ambiguous: the scalar
    // extractor will borrow the entire vector while the collection extractor
    // indexes it.  Reject such configurations early to avoid confusing
    // downstream type errors.
    // ---------------------------------------------------------------------
    use std::collections::HashMap;
    let mut anchor_usage: HashMap<&syn::Ident, Vec<&FieldKind>> = HashMap::new();
    for field in &ir.fields {
        if let Some(ident) = &field.attr.anchor_ident {
            anchor_usage.entry(ident).or_default().push(&field.kind);
        }
    }

    for (ident, kinds) in anchor_usage {
        let has_collection = kinds
            .iter()
            .any(|k| matches!(k, FieldKind::Array(_) | FieldKind::Vec));
        let has_scalar = kinds
            .iter()
            .any(|k| matches!(k, FieldKind::Single | FieldKind::Optional));
        if has_collection && has_scalar {
            return Err(Error::new(
                ir.struct_ident.span(),
                format!(
                    "anchor target `{}` is used by both collection and scalar fields; this is ambiguous. Use distinct anchors or convert the scalar field to an indexed form.",
                    ident
                ),
            ));
        }

        // An anchor identifier may only back **one** collection field (Vec or Array)
        let collection_fields: Vec<_> = kinds
            .iter()
            .filter(|k| matches!(k, FieldKind::Array(_) | FieldKind::Vec))
            .collect();

        if collection_fields.len() > 1 {
            return Err(Error::new(
                ir.struct_ident.span(),
                format!(
                    "anchor target `{}` is referenced by multiple collection fields; expected at most one Vec/Array with a 1-to-1 mapping to the accounts collection",
                    ident
                ),
            ));
        }
    }

    // ---------------------------------------------------------------------
    // Vec-related constraints.
    // ---------------------------------------------------------------------
    for field in &ir.fields {
        if let FieldKind::Vec = field.kind {
            match (field.attr.anchor_ident.is_some(), field.attr.rest) {
                // Vec + anchor but no rest → OK
                (true, false) => {}
                // Vec + anchor + rest → invalid
                (true, true) => {
                    return Err(Error::new(
                        field.span,
                        "Vec field cannot combine `anchor = <field>` with `rest` flag",
                    ));
                }
                // Vec + rest (no anchor) → OK
                (false, true) => {}
                // Vec without rest or anchor → invalid
                (false, false) => {
                    return Err(Error::new(
                        field.span,
                        "Vec field must be marked with `rest` flag: #[utxo(rest, ...)]",
                    ));
                }
            }
        } else {
            // Non-Vec field must not use `rest` flag.
            if field.attr.rest {
                return Err(Error::new(
                    field.span,
                    "`rest` flag is only allowed on Vec fields",
                ));
            }
        }
    }

    // ---------------------------------------------------------------------
    // `rest` field constraints: at most one, and must be last.
    // ---------------------------------------------------------------------
    let mut rest_field_span: Option<Span> = None;
    let mut rest_field_index: Option<usize> = None;
    for (idx, field) in ir.fields.iter().enumerate() {
        if field.attr.rest {
            if let Some(prev_span) = rest_field_span {
                return Err(Error::new(
                    prev_span,
                    "Multiple fields are marked with `rest`; only one `#[utxo(rest)]` field is allowed",
                ));
            }
            rest_field_span = Some(field.span);
            rest_field_index = Some(idx);
        }
    }

    if let Some(idx) = rest_field_index {
        if idx + 1 != ir.fields.len() {
            return Err(Error::new(
                rest_field_span.unwrap(),
                "The `#[utxo(rest)]` field must be the last field in the struct because UTXO order is now significant",
            ));
        }
    }

    // ---------------------------------------------------------------------
    // Incompatible rune constraints (e.g. `runes = "none"` with `rune_id`/`rune_amount`).
    // ---------------------------------------------------------------------
    for field in &ir.fields {
        if matches!(field.attr.runes, Some(crate::ir::RunesPresence::None))
            && (field.attr.rune_id_expr.is_some() || field.attr.rune_amount_expr.is_some())
        {
            return Err(Error::new(
                field.attr.span,
                "`runes = \"none\"` cannot be combined with `rune_id` or `rune_amount`",
            ));
        }

        // Prevent logically impossible combination of `runes = "some"` with
        // a zero `rune_amount` literal (the predicate would never match).
        if let (Some(crate::ir::RunesPresence::Some), Some(expr)) =
            (field.attr.runes, &field.attr.rune_amount_expr)
        {
            // Only analyse simple integer literals – if the amount is an expression we
            // cannot reason about its value at macro expansion time.
            if let syn::Expr::Lit(expr_lit) = expr {
                if let syn::Lit::Int(lit_int) = &expr_lit.lit {
                    if let Ok(v) = lit_int.base10_parse::<u128>() {
                        if v == 0 {
                            return Err(Error::new(
                                expr.span(),
                                "`runes = \"some\"` with `rune_amount = 0` is impossible – use `runes = \"none\"` or specify a positive amount",
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    fn ir_from(src: &str) -> crate::ir::DeriveInputIr {
        let di: syn::DeriveInput = syn::parse_str(src).unwrap();
        parse::derive_input_to_ir(&di).unwrap()
    }

    #[test]
    fn multiple_distinct_anchors_ok() {
        // Using different anchor targets should now be accepted.
        let code = r#"
            #[utxo_accounts(Accs)]
            struct S {
                #[utxo(anchor = acc1)]
                a: UtxoInfo,
                #[utxo(anchor = acc2)]
                b: UtxoInfo,
            }
        "#;
        let ir = ir_from(code);
        assert!(check(&ir).is_ok());
    }
}
