//! `#[error_code]` — emits a cheap `From<E> for Error` that wraps the enum
//! discriminant as `ProgramError::Custom(code)`. The `#[msg("...")]` helper
//! is IDL-only metadata — never allocated at runtime.
//!
//! Intentionally does **not** port v1's `AnchorError` struct (heap-allocated
//! name + msg + file/line). V2 programs route error text through the IDL;
//! runtime strings duplicate that at non-trivial CU cost.

use {
    proc_macro::TokenStream,
    proc_macro2::Span,
    quote::quote,
    syn::{
        parse_macro_input, spanned::Spanned, Attribute, Expr, ItemEnum, Lit, Meta, MetaNameValue,
    },
};

/// Default first error code. Matches v1's `ERROR_CODE_OFFSET`.
const DEFAULT_OFFSET: u32 = 6000;

pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let offset = parse_offset(args).unwrap_or(DEFAULT_OFFSET);
    let mut item = parse_macro_input!(input as ItemEnum);
    let name = item.ident.clone();

    let mut discriminator = 0;
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    for variant in item.variants.iter_mut() {
        let message = extract_msg(&variant.attrs);
        // Strip used `msg` attribute
        variant.attrs.retain(|a| !a.path().is_ident("msg"));
        if let Some((_, discr)) = &variant.discriminant {
            if let Some(discr) = parse_discrim(discr) {
                discriminator = discr;
            } else {
                errors.push(
                    syn::Error::new_spanned(discr, "discriminant must be a u32 literal")
                        .to_compile_error(),
                );
                continue;
            }
        }
        entries.push(ErrorEntry {
            name: variant.ident.to_string(),
            message,
            discriminator,
            span: variant.span(),
        });
        if let Some(next_discrim) = discriminator.checked_add(1) {
            discriminator = next_discrim;
        } else {
            errors
                .push(syn::Error::new_spanned(variant, "error code overflowed").to_compile_error());
            break;
        }
    }

    let idl_json = match build_idl_errors_json(&entries, offset) {
        Ok(json) => json,
        Err(e) => {
            errors.push(e.to_compile_error());
            Default::default()
        }
    };
    let idl_fn_name = quote::format_ident!(
        "__anchor_private_print_idl_errors_{}",
        name.to_string().to_lowercase()
    );

    let from_impl = quote! {
        impl From<#name> for anchor_lang_v2::Error {
            #[inline(always)]
            fn from(e: #name) -> Self {
                // Guarenteed not to overflow in `build_idl_errors_json`
                anchor_lang_v2::Error::Custom(e as u32 + #offset)
            }
        }
    };

    // `__idl_errors()` mirrors `__idl_accounts()` (lang-v2/derive/src/idl.rs):
    // a `pub fn` on the type returning the IDL JSON string. Lets the existing
    // `mod idl_tests { ... }` style suites parse the output with
    // `serde_json::from_str::<Vec<IdlErrorCode>>()` instead of capturing
    // stdout from the `__anchor_private_print_idl_errors_*` test fn.
    let idl_print = quote! {
        #[cfg(feature = "idl-build")]
        impl #name {
            #[doc(hidden)]
            pub fn __idl_errors() -> anchor_lang_v2::__alloc::string::String {
                anchor_lang_v2::__alloc::string::String::from(#idl_json)
            }
        }

        #[cfg(all(test, feature = "idl-build"))]
        #[test]
        fn #idl_fn_name() {
            println!("--- IDL begin errors ---");
            println!("{}", #name::__idl_errors());
            println!("--- IDL end errors ---");
        }
    };

    TokenStream::from(quote! {
        #[repr(u32)]
        #[derive(Clone, Copy)]
        #item

        #from_impl
        #idl_print
        #(#errors)*
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

fn parse_discrim(discrim: &Expr) -> Option<u32> {
    match discrim {
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

struct ErrorEntry {
    name: String,
    message: Option<String>,
    /// The specified or calculated Rust discriminator
    discriminator: u32,
    span: Span,
}

fn build_idl_errors_json(entries: &[ErrorEntry], offset: u32) -> Result<String, syn::Error> {
    let parts: Vec<String> = entries
        .iter()
        .map(|error| {
            let Some(code) = offset.checked_add(error.discriminator) else {
                return Err(syn::Error::new(
                    error.span,
                    "error code overflowed when adding offset",
                ));
            };
            let escaped_name = escape_json(&error.name);
            match &error.message {
                Some(m) => Ok(format!(
                    r#"{{"code":{},"name":"{}","msg":"{}"}}"#,
                    code,
                    escaped_name,
                    escape_json(m),
                )),
                None => Ok(format!(r#"{{"code":{},"name":"{}"}}"#, code, escaped_name)),
            }
        })
        .collect::<Result<_, _>>()?;
    Ok(format!("[{}]", parts.join(",")))
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
