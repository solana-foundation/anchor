//! Proc macros for `anchor-asm-v2`.
//!
//! The main entry point is `asm_program!` which takes type definitions
//! and assembly file paths in a single invocation, generating both the
//! Rust items and a `global_asm!` block with all const operands.
//!
//! ```ignore
//! anchor_asm_v2_macros::asm_program! {
//!     #[error_enum(prefix = "E")]
//!     pub enum ErrorCode {
//!         InvalidDiscriminant,
//!         InvalidInstructionLength,
//!     }
//!
//!     #[offsets(prefix = "CTR")]
//!     #[repr(C)]
//!     pub struct Counter {
//!         pub value: u64,
//!         pub bump: u8,
//!         pub _pad: [u8; 7],
//!     }
//!
//!     asm {
//!         "asm/errors.s",
//!         "asm/entrypoint.s",
//!     }
//! }
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token, Expr, Fields, Ident, Item, Lit, LitStr, Meta, Token,
};

// ---------------------------------------------------------------------------
// asm_program! — the single entry point
// ---------------------------------------------------------------------------

struct AsmProgram {
    items: Vec<AnnotatedItem>,
    files: Vec<LitStr>,
}

enum AnnotatedItem {
    ErrorEnum {
        prefix: String,
        item: syn::ItemEnum,
    },
    Discriminant {
        prefix: String,
        item: syn::ItemEnum,
    },
    Offsets {
        prefix: String,
        item: syn::ItemStruct,
    },
    /// Pass-through: items without a recognized #[...] annotation
    /// are emitted as-is (useful for helper structs, impls, etc.)
    Passthrough(Item),
}

impl Parse for AsmProgram {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();
        let mut files = Vec::new();

        while !input.is_empty() {
            // Check for `asm { ... }` block
            if input.peek(Ident) {
                let lookahead = input.fork();
                let ident: Ident = lookahead.parse()?;
                if ident == "asm" {
                    // Consume the real tokens
                    let _: Ident = input.parse()?;
                    let content;
                    braced!(content in input);
                    let file_list: Punctuated<LitStr, Token![,]> =
                        Punctuated::parse_terminated(&content)?;
                    files = file_list.into_iter().collect();
                    continue;
                }
            }

            // Parse a Rust item
            let item: Item = input.parse()?;

            match classify_item(item)? {
                classified => items.push(classified),
            }
        }

        Ok(AsmProgram { items, files })
    }
}

/// Look at the attributes on an item and classify it.
fn classify_item(item: Item) -> syn::Result<AnnotatedItem> {
    match item {
        Item::Enum(mut e) => {
            if let Some((kind, prefix)) = extract_asm_attr(&mut e.attrs) {
                match kind.as_str() {
                    "error_enum" => Ok(AnnotatedItem::ErrorEnum { prefix, item: e }),
                    "discriminant" => Ok(AnnotatedItem::Discriminant { prefix, item: e }),
                    other => Err(syn::Error::new_spanned(
                        &e.ident,
                        format!("unknown asm attribute: {other}"),
                    )),
                }
            } else {
                Ok(AnnotatedItem::Passthrough(Item::Enum(e)))
            }
        }
        Item::Struct(mut s) => {
            if let Some((kind, prefix)) = extract_asm_attr(&mut s.attrs) {
                match kind.as_str() {
                    "offsets" => Ok(AnnotatedItem::Offsets { prefix, item: s }),
                    other => Err(syn::Error::new_spanned(
                        &s.ident,
                        format!("unknown asm attribute: {other}"),
                    )),
                }
            } else {
                Ok(AnnotatedItem::Passthrough(Item::Struct(s)))
            }
        }
        other => Ok(AnnotatedItem::Passthrough(other)),
    }
}

/// Extract and remove `#[error_enum(...)]`, `#[discriminant(...)]`, or
/// `#[offsets(...)]` from an attribute list. Returns (kind, prefix).
fn extract_asm_attr(attrs: &mut Vec<syn::Attribute>) -> Option<(String, String)> {
    let known = ["error_enum", "discriminant", "offsets"];
    let pos = attrs.iter().position(|a| {
        a.path()
            .get_ident()
            .map(|id| known.contains(&id.to_string().as_str()))
            .unwrap_or(false)
    })?;
    let attr = attrs.remove(pos);
    let kind = attr.path().get_ident()?.to_string();
    let prefix = parse_prefix_from_meta(&attr).unwrap_or_else(|| default_prefix(&kind));
    Some((kind, prefix))
}

fn parse_prefix_from_meta(attr: &syn::Attribute) -> Option<String> {
    let meta = attr.meta.clone();
    if let Meta::List(list) = meta {
        let inner: Meta = syn::parse2(list.tokens).ok()?;
        if let Meta::NameValue(nv) = inner {
            if nv.path.is_ident("prefix") {
                if let Expr::Lit(lit) = &nv.value {
                    if let Lit::Str(s) = &lit.lit {
                        return Some(s.value());
                    }
                }
            }
        }
    }
    None
}

fn default_prefix(kind: &str) -> String {
    match kind {
        "error_enum" => "E".to_string(),
        "discriminant" => "DISC".to_string(),
        _ => String::new(),
    }
}

/// The main proc macro.
#[proc_macro]
pub fn asm_program(input: TokenStream) -> TokenStream {
    let program = syn::parse_macro_input!(input as AsmProgram);

    let mut rust_items = Vec::new();
    let mut const_operands: Vec<TokenStream2> = Vec::new();

    for item in &program.items {
        match item {
            AnnotatedItem::ErrorEnum { prefix, item } => {
                rust_items.push(quote! { #item });
                for (i, v) in item.variants.iter().enumerate() {
                    let name = format_ident!(
                        "{}_{}",
                        prefix,
                        to_screaming_snake(&v.ident.to_string())
                    );
                    let val = (i + 1) as i32;
                    const_operands.push(quote! { #name = const #val, });
                }
            }
            AnnotatedItem::Discriminant { prefix, item } => {
                rust_items.push(quote! { #item });
                for (i, v) in item.variants.iter().enumerate() {
                    let name = format_ident!(
                        "{}_{}",
                        prefix,
                        to_screaming_snake(&v.ident.to_string())
                    );
                    let val = i as u32;
                    const_operands.push(quote! { #name = const #val, });
                }
            }
            AnnotatedItem::Offsets { prefix, item } => {
                rust_items.push(quote! { #item });
                let struct_name = &item.ident;
                if let Fields::Named(fields) = &item.fields {
                    for field in &fields.named {
                        let field_name = field.ident.as_ref().unwrap();
                        if field_name.to_string().starts_with('_') {
                            continue;
                        }
                        let const_name = format_ident!(
                            "{}_{}",
                            prefix,
                            to_screaming_snake(&field_name.to_string())
                        );
                        const_operands.push(quote! {
                            #const_name = const core::mem::offset_of!(#struct_name, #field_name) as i32,
                        });
                    }
                    let size_name = format_ident!("{}_SIZE", prefix);
                    const_operands.push(quote! {
                        #size_name = const core::mem::size_of::<#struct_name>() as i32,
                    });
                }
            }
            AnnotatedItem::Passthrough(item) => {
                rust_items.push(quote! { #item });
            }
        }
    }

    let file_includes: Vec<_> = program
        .files
        .iter()
        .map(|f| quote! { include_str!(#f), })
        .collect();

    let expanded = quote! {
        #(#rust_items)*

        core::arch::global_asm!(
            #(#file_includes)*
            #(#const_operands)*
        );
    };

    expanded.into()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn to_screaming_snake(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            let prev = s.chars().nth(i - 1).unwrap_or('_');
            let next = s.chars().nth(i + 1);
            if prev.is_lowercase()
                || (prev.is_uppercase() && next.map_or(false, |n| n.is_lowercase()))
            {
                result.push('_');
            }
        }
        result.push(c.to_ascii_uppercase());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screaming_snake() {
        assert_eq!(to_screaming_snake("InvalidDiscriminant"), "INVALID_DISCRIMINANT");
        assert_eq!(to_screaming_snake("RegisterMarket"), "REGISTER_MARKET");
        assert_eq!(to_screaming_snake("BaseVaultHasData"), "BASE_VAULT_HAS_DATA");
        assert_eq!(to_screaming_snake("UserHasData"), "USER_HAS_DATA");
    }
}
