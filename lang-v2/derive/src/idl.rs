//! IDL generation helpers.

use {
    proc_macro2::TokenStream as TokenStream2,
    quote::quote,
    syn::{Expr, Lit, Type},
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
    /// Names of sibling fields whose `has_one` chain targets this field.
    /// Emitted as `"relations":[...]`. Matches v1's semantics: relations
    /// live on the *target* account (the account being referenced), not
    /// the source — see `lang/syn/src/idl/accounts.rs::get_relations`.
    pub relations: Vec<&'a str>,
    /// `#[doc = "..."]` lines on the field, in source order. Emitted as
    /// `"docs":[...]`.
    pub docs: &'a [String],
    /// Pre-built `pda: {...}` object (JSON body, no leading comma). `None`
    /// when the field has no `seeds = [...]` attr.
    pub pda_json: Option<String>,
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
            let relations_json = if f.relations.is_empty() {
                String::new()
            } else {
                let list: Vec<String> =
                    f.relations.iter().map(|r| format!("\"{r}\"")).collect();
                format!(",\"relations\":[{}]", list.join(","))
            };
            let docs_json = if f.docs.is_empty() {
                String::new()
            } else {
                format!(",\"docs\":{}", docs_to_json_array(f.docs))
            };
            let pda_json = f
                .pda_json
                .as_ref()
                .map(|p| format!(",\"pda\":{p}"))
                .unwrap_or_default();
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
                            "{{\"name\":\"{}\"{}{}{}{}{}{}{}}}",
                            #name,
                            #writable_json,
                            __signer_json,
                            __addr_json,
                            #optional_json,
                            #relations_json,
                            #docs_json,
                            #pda_json,
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
                        "{{\"name\":\"{}\"{}{}{}{}{}{}}}",
                        #name,
                        #writable_json,
                        #signer_json,
                        #optional_json,
                        #relations_json,
                        #docs_json,
                        #pda_json,
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

/// Zero-copy / borsh mode tag passed down from the `#[account]` / `#[event]`
/// call sites. The spec (`idl/spec/src/lib.rs:180-216`) models this as the
/// pair `(IdlSerialization, Option<IdlRepr>)`, but both fields are tightly
/// coupled — bytemuck always pairs with `repr(C)` in our codegen, and borsh
/// carries no repr — so we collapse them into a single enum and expand both
/// fields at emission time.
#[derive(Clone, Copy)]
pub enum TypeKind {
    /// Default borsh layout. Spec `skip_serializing_if`s both fields at the
    /// default value, so nothing extra gets emitted.
    Borsh,
    /// `bytemuck` Pod + `repr(C)`. Both fields show up in the JSON.
    BytemuckRepr,
}

/// Build IDL type definition JSON from struct fields. `docs` is the list of
/// `#[doc = "..."]` lines scraped from the struct-level attrs; each named
/// field also contributes its own `docs` array from its own attrs.
///
/// `kind` selects the `serialization` / `repr` metadata emitted onto the
/// type definition. Zero-copy `#[account]` / default `#[event]` pass
/// `TypeKind::BytemuckRepr`; their borsh-mode counterparts pass
/// `TypeKind::Borsh` (the default, which suppresses both fields).
pub fn build_type_json(
    name: &str,
    disc: &[u8],
    docs: &[String],
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    kind: TypeKind,
) -> String {
    let disc_json = disc_json(disc);
    let docs_json = if docs.is_empty() {
        String::new()
    } else {
        format!(",\"docs\":{}", docs_to_json_array(docs))
    };
    // `IdlSerialization` / `IdlRepr` (spec:190-216). `Borsh` is the default
    // on both the tagged enum and the `Option<IdlRepr>`, so we splice
    // nothing and let the serde-default path handle it. `bytemuckUnsafe`
    // deliberately isn't emitted — its semantics aren't nailed down in v2.
    let serialization_repr_json = match kind {
        TypeKind::Borsh => String::new(),
        TypeKind::BytemuckRepr => {
            ",\"serialization\":\"bytemuck\",\"repr\":{\"kind\":\"c\"}".to_string()
        }
    };
    let field_jsons: Vec<String> = fields
        .iter()
        .map(|f| {
            let fname = f.ident.as_ref().unwrap().to_string();
            let ftype = rust_type_to_idl(&f.ty);
            let field_docs = extract_doc_lines(&f.attrs);
            let field_docs_json = if field_docs.is_empty() {
                String::new()
            } else {
                format!(",\"docs\":{}", docs_to_json_array(&field_docs))
            };
            format!("{{\"name\":\"{fname}\"{field_docs_json},\"type\":{ftype}}}")
        })
        .collect();
    format!(
        "{{\"name\":\"{name}\",\"discriminator\":{disc_json}{docs_json}{serialization_repr_json},\"type\":{{\"kind\":\"struct\",\"\
         fields\":[{}]}}}}",
        field_jsons.join(",")
    )
}

// ---------------------------------------------------------------------------
// Docs extraction
// ---------------------------------------------------------------------------

/// Extract `#[doc = "..."]` lines from a list of attributes. `/// foo`
/// desugars to `#[doc = " foo"]` — the compiler inserts a single leading
/// space that we strip so IDL consumers don't see extra indentation.
pub fn extract_doc_lines(attrs: &[syn::Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }
            if let syn::Meta::NameValue(nv) = &attr.meta {
                if let Expr::Lit(lit) = &nv.value {
                    if let Lit::Str(s) = &lit.lit {
                        let v = s.value();
                        return Some(
                            v.strip_prefix(' ').map(str::to_owned).unwrap_or(v),
                        );
                    }
                }
            }
            None
        })
        .collect()
}

/// Serialize a list of doc lines into a JSON array. Pulling `serde_json`
/// into a proc-macro crate is overkill for what amounts to a 7-byte escape
/// table, so the escaping is inlined.
pub fn docs_to_json_array(docs: &[String]) -> String {
    let parts: Vec<String> = docs
        .iter()
        .map(|d| format!("\"{}\"", escape_json_string(d)))
        .collect();
    format!("[{}]", parts.join(","))
}

fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Seed classification (Part E — `pda: {...}` emission)
// ---------------------------------------------------------------------------

/// Classify a single seed expression into one of the `IdlSeed` variants
/// (spec:111-134) and return a pre-built JSON object string ready to splice
/// into a `seeds` array.
///
/// Recognized shapes:
/// - byte literal (`b"counter"`)              → `{"kind":"const","value":[...]}`
/// - byte-array literal (`[1, 2, 3]`)         → `{"kind":"const","value":[...]}`
/// - string literal (`"counter"`)             → `{"kind":"const","value":[<bytes>]}`
/// - `"literal".as_bytes()`                   → `{"kind":"const","value":[...]}`
/// - account field ref (`user` bare ident,
///   `user.key().as_ref()`, `user.address().as_ref()`,
///   `user.as_ref()`) with `user` in `field_names`
///                                            → `{"kind":"account","path":"user"}`
/// - instruction arg ref (`nonce` bare ident,
///   `nonce.to_le_bytes()`, `nonce.as_ref()`)
///   with `nonce` in `ix_arg_names`
///                                            → `{"kind":"arg","path":"nonce"}`
/// - anything else (e.g. `Some::Path::call()`) → `const` with empty value
///   + eprintln warning so the anchor build CLI surfaces it.
pub fn classify_seed(
    expr: &Expr,
    field_names: &[String],
    ix_arg_names: &[String],
) -> String {
    // Peel `&<inner>` wrappers — they're common in seed expressions and
    // always transparent to classification.
    let mut cur = expr;
    while let Expr::Reference(r) = cur {
        cur = &r.expr;
    }

    // Byte / string / byte-lit literal.
    if let Expr::Lit(lit) = cur {
        match &lit.lit {
            Lit::ByteStr(bs) => return const_seed_json(&bs.value()),
            Lit::Str(s) => return const_seed_json(s.value().as_bytes()),
            Lit::Byte(b) => return const_seed_json(&[b.value()]),
            _ => {}
        }
    }

    // Array literal with fully-u8 elements: [1, 2, 3]
    if let Expr::Array(arr) = cur {
        let mut bytes: Option<Vec<u8>> = Some(Vec::with_capacity(arr.elems.len()));
        for e in &arr.elems {
            if let Expr::Lit(syn::ExprLit {
                lit: Lit::Int(i), ..
            }) = e
            {
                if let Ok(v) = i.base10_parse::<u8>() {
                    bytes.as_mut().unwrap().push(v);
                    continue;
                }
            }
            bytes = None;
            break;
        }
        if let Some(b) = bytes {
            return const_seed_json(&b);
        }
    }

    // Bare ident — field ref wins, then ix arg.
    if let Expr::Path(ep) = cur {
        if ep.qself.is_none() && ep.path.segments.len() == 1 && ep.path.leading_colon.is_none() {
            let seg = &ep.path.segments[0];
            if seg.arguments.is_empty() {
                let name = seg.ident.to_string();
                if field_names.contains(&name) {
                    return account_seed_json(&name);
                }
                if ix_arg_names.contains(&name) {
                    return arg_seed_json(&name);
                }
            }
        }
    }

    // Method-call / field-access chains: walk to the receiver's bare
    // ident. Handles `foo.bar()`, `foo.bar.baz`, `foo.key().as_ref()`,
    // `foo.to_le_bytes()`, `foo[0]`, `(foo)`, etc.
    if let Some(root) = receiver_root_ident(cur) {
        if field_names.contains(&root) {
            return account_seed_json(&root);
        }
        if ix_arg_names.contains(&root) {
            return arg_seed_json(&root);
        }
    }

    // `"literal".as_bytes()` — receiver is a string literal, not an
    // ident, so the walk above missed it. Pick it up here.
    if let Expr::MethodCall(mc) = cur {
        if let Expr::Lit(syn::ExprLit {
            lit: Lit::Str(s), ..
        }) = &*mc.receiver
        {
            if mc.method == "as_bytes" {
                return const_seed_json(s.value().as_bytes());
            }
        }
    }

    // Unknown — emit a warning (visible in `cargo build` output) and keep
    // emission valid with an empty const so downstream tooling doesn't
    // choke on malformed JSON.
    eprintln!(
        "anchor-v2 idl: unable to classify seed expression `{}`; emitting empty const",
        quote!(#expr)
    );
    const_seed_json(&[])
}

/// Walk down a method-call / field-access / index chain and return the
/// bare ident at its root, if any. `foo.key().as_ref()` → `foo`;
/// `foo.bar.baz` → `foo`; `foo[0]` → `foo`; `(foo)` → `foo`.
fn receiver_root_ident(expr: &Expr) -> Option<String> {
    let mut cur = expr;
    loop {
        match cur {
            Expr::MethodCall(mc) => cur = &mc.receiver,
            Expr::Field(fa) => cur = &fa.base,
            Expr::Index(ix) => cur = &ix.expr,
            Expr::Paren(p) => cur = &p.expr,
            Expr::Reference(r) => cur = &r.expr,
            Expr::Path(ep)
                if ep.qself.is_none()
                    && ep.path.segments.len() == 1
                    && ep.path.leading_colon.is_none()
                    && ep.path.segments[0].arguments.is_empty() =>
            {
                return Some(ep.path.segments[0].ident.to_string());
            }
            _ => return None,
        }
    }
}

fn const_seed_json(bytes: &[u8]) -> String {
    let values: Vec<String> = bytes.iter().map(|b| b.to_string()).collect();
    format!("{{\"kind\":\"const\",\"value\":[{}]}}", values.join(","))
}

fn account_seed_json(path: &str) -> String {
    format!("{{\"kind\":\"account\",\"path\":\"{path}\"}}")
}

fn arg_seed_json(path: &str) -> String {
    format!("{{\"kind\":\"arg\",\"path\":\"{path}\"}}")
}

/// Assemble the `pda: {...}` object body from a field's classified seeds
/// plus optional program override. Returns the JSON object (without the
/// leading `,"pda":` — that's spliced by `build_accounts_emission`).
pub fn pda_object_json(seeds: &[String], program: Option<&String>) -> String {
    let seeds_arr = format!("[{}]", seeds.join(","));
    match program {
        Some(p) => format!("{{\"seeds\":{seeds_arr},\"program\":{p}}}"),
        None => format!("{{\"seeds\":{seeds_arr}}}"),
    }
}
