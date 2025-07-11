#![allow(dead_code)]
//! Intermediate representation (IR) for the `UtxoParser` derive macro.
//!
//! By converting the incoming `syn::DeriveInput` into these plain Rust
//! structures first, we decouple parsing/validation from code-generation and
//! make unit testing trivial.

use proc_macro2::Span;
use syn::{Ident, Type};

/// What kind of UTXO collection a field represents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldKind {
    /// A single `UtxoInfo` value.
    Single,
    /// A fixed-length array `[UtxoInfo; N]`.
    Array(usize),
    /// A catch-all `Vec<UtxoInfo>`.
    Vec,
    /// An optional `Option<UtxoInfo>` value.
    Optional,
}

/// Presence predicate coming from `runes = "..."`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunesPresence {
    None,
    Some,
    Any,
}

/// Data extracted from a single `#[utxo(..)]` attribute.
#[derive(Debug, Clone)]
pub struct UtxoAttr {
    /// Match only UTXOs whose `value` equals this amount (satoshis).
    pub value: Option<syn::Expr>,
    /// Constraints on rune presence (none / some / any).
    pub runes: Option<RunesPresence>,
    /// Expression AST for a specific rune id check.
    pub rune_id_expr: Option<syn::Expr>,
    /// Expression AST for a specific rune amount check.
    pub rune_amount_expr: Option<syn::Expr>,
    /// Whether this Vec field should capture the remaining inputs.
    pub rest: bool,
    /// Identifier of the accounts struct field to anchor against, if any.
    pub anchor_ident: Option<Ident>,
    /// Span of the attribute – kept for diagnostics.
    pub span: Span,
}

impl Default for UtxoAttr {
    fn default() -> Self {
        Self {
            value: None,
            runes: None,
            rune_id_expr: None,
            rune_amount_expr: None,
            rest: false,
            anchor_ident: None,
            span: Span::call_site(),
        }
    }
}

/// Representation of a single struct field after parsing.
#[derive(Debug, Clone)]
pub struct Field {
    pub ident: Ident,
    pub kind: FieldKind,
    pub ty: Type,
    pub attr: UtxoAttr,
    pub span: Span,
}

/// Parsed, high-level description of the entire derive input.
#[derive(Debug, Clone)]
pub struct DeriveInputIr {
    pub struct_ident: Ident,
    pub generics: syn::Generics,
    pub accounts_ty: Type,
    pub fields: Vec<Field>,
}
