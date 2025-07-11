#![allow(dead_code)]
//! Helpers for building predicate `TokenStream`s from `UtxoAttr`.

use crate::ir::{RunesPresence, UtxoAttr};
use quote::quote;

/// Build a boolean expression over a `utxo` variable matching the constraints
/// described by `attr`.
pub fn build(attr: &UtxoAttr) -> proc_macro2::TokenStream {
    let mut parts: Vec<proc_macro2::TokenStream> = Vec::new();

    // value predicate: allow arbitrary expressions (consts, arithmetic, etc.).
    if let Some(value_expr) = &attr.value {
        parts.push(quote! { utxo.value == (#value_expr) });
    }

    // runes presence
    match attr.runes {
        Some(RunesPresence::None) => parts.push(quote! { utxo.rune_entry_count() == 0 }),
        Some(RunesPresence::Some) => parts.push(quote! { utxo.rune_entry_count() > 0 }),
        _ => {}
    }

    // rune id / amount combinations
    match (&attr.rune_id_expr, &attr.rune_amount_expr) {
        (Some(id), Some(amount)) => {
            parts.push(quote! { utxo.contains_exact_rune(&(#id), (#amount) as u128) });
        }
        (Some(id), None) => {
            parts.push(quote! { utxo.rune_amount(&(#id)).is_some() });
        }
        (None, Some(amount)) => {
            parts.push(quote! { utxo.total_rune_amount() == (#amount) as u128 });
        }
        _ => {}
    }

    if parts.is_empty() {
        quote! { true }
    } else {
        quote! { #( #parts )&&* }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{RunesPresence, UtxoAttr};
    use syn::parse_quote;

    #[test]
    fn predicate_contains_parts() {
        let mut a = UtxoAttr::default();
        a.value = Some(parse_quote!(10));
        a.runes = Some(RunesPresence::Some);
        let ts = build(&a);
        let s = ts.to_string().replace(" ", "");
        assert!(s.contains("utxo.value==(10)"));
        assert!(s.contains("utxo.rune_entry_count()>0"));
    }
}
