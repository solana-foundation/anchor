//! IDL generation helpers.

use {
    proc_macro2::TokenStream as TokenStream2,
    quote::quote,
    syn::Type,
};

/// Convert a Rust type to its IDL JSON representation.
pub fn rust_type_to_idl(ty: &Type) -> String {
    type_str_to_idl(&quote!(#ty).to_string().replace(' ', ""))
}

/// Convert a stringified Rust type to IDL JSON.
fn type_str_to_idl(s: &str) -> String {
    // Strip lifetimes and leading `&` (reference) so `&'a [u64]` → `[u64]`.
    let s = strip_ref_and_lifetime(s);
    let s = s.as_str();
    match s {
        "u8" | "u16" | "u32" | "u64" | "u128" | "i8" | "i16" | "i32" | "i64" | "i128" | "bool" => {
            format!("\"{s}\"")
        }
        "String" | "string" | "str" => "\"string\"".into(),
        "Pubkey" | "Address" | "pubkey" => "\"pubkey\"".into(),
        "bytes" => "\"bytes\"".into(),
        // `&[u8]` → bytes
        "[u8]" => "\"bytes\"".into(),
        // `&[T]` slice → vec<T>
        _ if s.starts_with('[') && s.ends_with(']') && !s.contains(';') => {
            let inner = &s[1..s.len() - 1];
            format!("{{\"vec\":{}}}", type_str_to_idl(inner))
        }
        // `[T; N]` array
        _ if s.starts_with('[') && s.ends_with(']') && s.contains(';') => {
            let inner = &s[1..s.len() - 1];
            if let Some((ty_part, n_part)) = inner.split_once(';') {
                let ty_json = type_str_to_idl(ty_part);
                // Try to parse as integer literal; if const expression, use 0 as placeholder
                let size = n_part.trim().parse::<usize>().unwrap_or(0);
                format!("{{\"array\":[{ty_json},{size}]}}")
            } else {
                format!("{{\"defined\":{{\"name\":\"{s}\"}}}}")
            }
        }
        _ if s.starts_with("Vec<") => {
            let inner = s.strip_prefix("Vec<").unwrap().strip_suffix('>').unwrap();
            format!("{{\"vec\":{}}}", type_str_to_idl(inner))
        }
        _ if s.starts_with("Option<") => {
            let inner = s
                .strip_prefix("Option<")
                .unwrap()
                .strip_suffix('>')
                .unwrap();
            format!("{{\"option\":{}}}", type_str_to_idl(inner))
        }
        _ if s.starts_with("Box<") => {
            let inner = s.strip_prefix("Box<").unwrap().strip_suffix('>').unwrap();
            type_str_to_idl(inner)
        }
        other => format!("{{\"defined\":{{\"name\":\"{other}\"}}}}"),
    }
}

/// Strip a leading `&` reference and any `'lifetime` annotation from a type
/// string, so `&'a [u64]` → `[u64]`. Leaves nested types alone.
fn strip_ref_and_lifetime(s: &str) -> String {
    let s = s.trim();
    // Remove leading `&` and optional `mut`
    let s = s.strip_prefix('&').unwrap_or(s).trim_start();
    // Remove lifetime annotation like `'a` (stop at the next whitespace/'[')
    let s = if let Some(rest) = s.strip_prefix('\'') {
        // Drop up to the next whitespace
        match rest.find(|c: char| c.is_whitespace() || c == '[' || c == ',') {
            Some(pos) => rest[pos..].trim_start().to_owned(),
            None => String::new(),
        }
    } else {
        s.to_owned()
    };
    s.trim_start_matches("mut ").trim().to_owned()
}

/// Per-field input to the runtime `__idl_accounts()` emission. See
/// [`build_accounts_emission`].
pub struct AccountsJsonField<'a> {
    pub name: &'a str,
    pub writable: bool,
    pub init_signer: bool,
    /// True when the field type is `Option<T>`. Surfaces as
    /// `"optional":true` in the emitted JSON (matches
    /// `IdlInstructionAccount.optional` in `idl/spec/src/lib.rs:89`).
    pub is_optional: bool,
    /// The wrapper `Type` (post-`Option` unwrap) whose trait consts we
    /// dispatch on at runtime. Should match `AccountField::idl_field_ty`.
    pub field_ty: &'a Option<Type>,
}

/// Build a `fn __idl_accounts() -> alloc::string::String` body that assembles
/// the accounts JSON at runtime by reading `<Ty as IdlAccountType>::
/// __IDL_IS_SIGNER / __IDL_ADDRESS`. Compile-time-known flags (writable,
/// init-signer) are baked into the format literals so no runtime work is
/// done for them.
///
/// Runtime assembly (rather than a `&'static str` baked at macro time) is
/// the one concession needed to let the wrapper type's trait const drive
/// per-field signer / address — the const values aren't visible to the
/// macro. Cost is paid once when `anchor idl build` invokes the test.
pub fn build_accounts_emission(fields: &[AccountsJsonField<'_>]) -> TokenStream2 {
    let parts: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let name = f.name;
            let writable_json = if f.writable {
                ",\"writable\":true"
            } else {
                ""
            };
            let optional_json = if f.is_optional {
                ",\"optional\":true"
            } else {
                ""
            };
            let init_signer = f.init_signer;
            if let Some(ty) = f.field_ty {
                quote! {
                    {
                        // Trait-const OR compile-time init_signer flag.
                        // Kept separate so a Signer + init-without-seeds
                        // combo still renders exactly one `"signer":true`.
                        let __signer = <#ty as anchor_lang_v2::IdlAccountType>::__IDL_IS_SIGNER
                            || #init_signer;
                        let __addr = <#ty as anchor_lang_v2::IdlAccountType>::__IDL_ADDRESS;
                        let __signer_json: &str = if __signer { ",\"signer\":true" } else { "" };
                        let __addr_json: anchor_lang_v2::__alloc::string::String = match __addr {
                            Some(a) => anchor_lang_v2::__alloc::format!(",\"address\":\"{}\"", a),
                            None => anchor_lang_v2::__alloc::string::String::new(),
                        };
                        anchor_lang_v2::__alloc::format!(
                            "{{\"name\":\"{}\"{}{}{}{}}}",
                            #name,
                            #writable_json,
                            __signer_json,
                            __addr_json,
                            #optional_json,
                        )
                    }
                }
            } else {
                // Defensive fallback for non-`Type::Path` field types —
                // can't resolve the trait, so we emit only compile-time
                // flags. Never triggers for valid Accounts structs.
                let signer_json = if init_signer { ",\"signer\":true" } else { "" };
                quote! {
                    anchor_lang_v2::__alloc::format!(
                        "{{\"name\":\"{}\"{}{}{}}}",
                        #name,
                        #writable_json,
                        #signer_json,
                        #optional_json,
                    )
                }
            }
        })
        .collect();

    quote! {
        pub fn __idl_accounts() -> anchor_lang_v2::__alloc::string::String {
            let __parts: anchor_lang_v2::__alloc::vec::Vec<
                anchor_lang_v2::__alloc::string::String
            > = anchor_lang_v2::__alloc::vec![#(#parts),*];
            let mut __s = anchor_lang_v2::__alloc::string::String::from("[");
            let mut __first = true;
            for __p in &__parts {
                if !__first { __s.push(','); }
                __first = false;
                __s.push_str(__p);
            }
            __s.push(']');
            __s
        }
    }
}

/// Build IDL instruction args JSON from handler parameters.
pub fn build_args_json(args: &[(&syn::Ident, &Type)]) -> String {
    let parts: Vec<String> = args
        .iter()
        .map(|(name, ty)| {
            let ty_json = rust_type_to_idl(ty);
            format!("{{\"name\":\"{name}\",\"type\":{ty_json}}}")
        })
        .collect();
    format!("[{}]", parts.join(","))
}

/// Build discriminator JSON from hash bytes.
pub fn disc_json(disc_bytes: &[u8]) -> String {
    let parts: Vec<String> = disc_bytes.iter().map(|b| b.to_string()).collect();
    format!("[{}]", parts.join(","))
}

/// Build IDL type definition JSON from struct fields.
pub fn build_type_json(
    name: &str,
    disc: &[u8],
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> String {
    let disc_json = disc_json(disc);
    let field_jsons: Vec<String> = fields
        .iter()
        .map(|f| {
            let fname = f.ident.as_ref().unwrap().to_string();
            let ftype = rust_type_to_idl(&f.ty);
            format!("{{\"name\":\"{fname}\",\"type\":{ftype}}}")
        })
        .collect();
    format!(
        "{{\"name\":\"{name}\",\"discriminator\":{disc_json},\"type\":{{\"kind\":\"struct\",\"\
         fields\":[{}]}}}}",
        field_jsons.join(",")
    )
}
