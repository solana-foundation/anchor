//! `#[error_code]` — emits a cheap `From<E> for Error` that wraps the enum
//! discriminant as `ProgramError::Custom(code)`. The `#[msg("...")]` helper
//! is IDL-only metadata — never allocated at runtime.
//!
//! Intentionally does **not** port v1's `AnchorError` struct (heap-allocated
//! name + msg + file/line). V2 programs route error text through the IDL;
//! runtime strings duplicate that at non-trivial CU cost.

use {
    proc_macro::TokenStream,
    proc_macro2::TokenStream as TokenStream2,
    quote::quote,
    syn::{parse_macro_input, Attribute, Expr, ItemEnum, Lit, Meta, MetaNameValue},
};

/// Default first error code. Matches v1's `ERROR_CODE_OFFSET`.
const DEFAULT_OFFSET: u32 = 6000;

pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let offset = parse_offset(args).unwrap_or(DEFAULT_OFFSET);
    let mut item = parse_macro_input!(input as ItemEnum);
    let name = item.ident.clone();

    // Collect (variant_name, msg) pairs and strip `#[msg(...)]` from the enum
    // so the emitted enum doesn't carry an attribute that no downstream
    // attribute macro recognizes.
    let mut entries: Vec<(String, Option<String>)> = Vec::new();
    for variant in item.variants.iter_mut() {
        let msg = extract_msg(&variant.attrs);
        variant.attrs.retain(|a| !a.path().is_ident("msg"));
        entries.push((variant.ident.to_string(), msg));
    }

    let idl_json = build_idl_errors_json(&entries, offset);
    let idl_fn_name = quote::format_ident!(
        "__anchor_private_print_idl_errors_{}",
        name.to_string().to_lowercase()
    );

    let from_impl = quote! {
        impl From<#name> for anchor_lang_v2::Error {
            #[inline(always)]
            fn from(e: #name) -> Self {
                anchor_lang_v2::Error::Custom(e as u32 + #offset)
            }
        }
    };

    let idl_print = quote! {
        #[cfg(all(test, feature = "idl-build"))]
        #[test]
        fn #idl_fn_name() {
            println!("--- IDL begin errors ---");
            println!("{}", #idl_json);
            println!("--- IDL end errors ---");
        }
    };

    TokenStream::from(quote! {
        #[repr(u32)]
        #[derive(Clone, Copy)]
        #item

        #from_impl
        #idl_print
    })
}

fn parse_offset(args: TokenStream) -> Option<u32> {
    if args.is_empty() {
        return None;
    }
    let meta: MetaNameValue = syn::parse(args).ok()?;
    if !meta.path.is_ident("offset") {
        return None;
    }
    match meta.value {
        Expr::Lit(syn::ExprLit {
            lit: Lit::Int(i), ..
        }) => i.base10_parse::<u32>().ok(),
        _ => None,
    }
}

fn extract_msg(attrs: &[Attribute]) -> Option<String> {
    attrs.iter().find_map(|a| {
        if !a.path().is_ident("msg") {
            return None;
        }
        // `#[msg("text")]` parses as a list-style attribute.
        match &a.meta {
            Meta::List(list) => {
                let lit: Lit = syn::parse2(list.tokens.clone()).ok()?;
                if let Lit::Str(s) = lit {
                    Some(s.value())
                } else {
                    None
                }
            }
            _ => None,
        }
    })
}

fn build_idl_errors_json(entries: &[(String, Option<String>)], offset: u32) -> TokenStream2 {
    let parts: Vec<String> = entries
        .iter()
        .enumerate()
        .map(|(i, (name, msg))| {
            let code = offset + i as u32;
            let escaped_name = escape_json(name);
            match msg {
                Some(m) => format!(
                    r#"{{"code":{},"name":"{}","msg":"{}"}}"#,
                    code,
                    escaped_name,
                    escape_json(m),
                ),
                None => format!(r#"{{"code":{},"name":"{}"}}"#, code, escaped_name),
            }
        })
        .collect();
    let json = format!("[{}]", parts.join(","));
    quote! { #json }
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
