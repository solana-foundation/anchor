//! IDL generation helpers.
//!
//! All macro-time JSON construction goes through `serde_json::Value` and
//! serializes once at the boundary. Hand-rolled `format!()` string splicing
//! is a footgun — an unescaped quote in a doc comment or a malformed
//! `Custom(String)` shape would silently produce invalid JSON, and the
//! failure surfaces far downstream as "unknown variant" or parser
//! crashes in TS clients. Using `serde_json::json!()` and `Value` gets
//! escaping, composition, and round-trip fidelity for free.
//!
//! The one exception is [`build_accounts_emission`]: it generates a runtime
//! `__idl_accounts()` function that assembles JSON at test time (inside the
//! program crate, not the macro), and pulling `serde_json` into the user's
//! program is not worth it — those format! calls are controlled and tested.

use {
    proc_macro2::TokenStream as TokenStream2,
    quote::quote,
    serde_json::{json, Value},
    syn::{Expr, Lit, Type},
};

/// Convert a Rust type to its IDL JSON representation (as a `serde_json`
/// value ready to splice into a containing `Value`). See [`rust_type_to_idl`]
/// for the stringified convenience wrapper.
pub fn rust_type_to_idl_value(ty: &Type) -> Value {
    type_str_to_idl_value(&quote!(#ty).to_string().replace(' ', ""))
}

/// String-returning convenience wrapper around [`rust_type_to_idl_value`].
/// Kept for callers that splice the result into runtime `format!()` templates.
pub fn rust_type_to_idl(ty: &Type) -> String {
    rust_type_to_idl_value(ty).to_string()
}

/// Convert a stringified Rust type to an IDL JSON value.
fn type_str_to_idl_value(s: &str) -> Value {
    let s = strip_ref_and_lifetime(s);
    let s = s.as_str();
    match s {
        "u8" | "u16" | "u32" | "u64" | "u128" | "i8" | "i16" | "i32" | "i64" | "i128" | "bool" => {
            Value::String(s.to_owned())
        }
        // Pod wrappers (`lang-v2/src/pod.rs`) drop alignment to 1 so the type
        // fits in `repr(C)` zero-copy accounts without padding, but the
        // on-disk byte representation is bit-identical to the corresponding
        // primitive (8 bytes LE for `PodU64`, one byte for `PodBool`, etc.).
        // Report them as primitives so the TS coder's default borsh path
        // decodes them without a registered `types[]` entry or hand-rolled
        // readers. `PodVec<T, N>` stays defined — its layout is non-trivial.
        "PodU16" => Value::String("u16".into()),
        "PodU32" => Value::String("u32".into()),
        "PodU64" => Value::String("u64".into()),
        "PodU128" => Value::String("u128".into()),
        "PodI16" => Value::String("i16".into()),
        "PodI32" => Value::String("i32".into()),
        "PodI64" => Value::String("i64".into()),
        "PodI128" => Value::String("i128".into()),
        "PodBool" => Value::String("bool".into()),
        "String" | "string" | "str" => Value::String("string".into()),
        "Pubkey" | "Address" | "pubkey" => Value::String("pubkey".into()),
        "bytes" | "[u8]" => Value::String("bytes".into()),
        // `&[T]` slice (no `;`) → vec<T>
        _ if s.starts_with('[') && s.ends_with(']') && !s.contains(';') => {
            let inner = &s[1..s.len() - 1];
            json!({ "vec": type_str_to_idl_value(inner) })
        }
        // `[T; N]` array
        _ if s.starts_with('[') && s.ends_with(']') && s.contains(';') => {
            let inner = &s[1..s.len() - 1];
            if let Some((ty_part, n_part)) = inner.split_once(';') {
                let ty_json = type_str_to_idl_value(ty_part);
                // Const expressions that aren't plain integer literals fall
                // through to 0 as a placeholder — matches the prior behavior.
                let size = n_part.trim().parse::<usize>().unwrap_or(0);
                json!({ "array": [ty_json, size] })
            } else {
                json!({ "defined": { "name": s } })
            }
        }
        _ if s.starts_with("Vec<") => {
            let inner = s
                .strip_prefix("Vec<")
                .and_then(|s| s.strip_suffix('>'))
                .expect("syn-generated type string has balanced angle brackets");
            json!({ "vec": type_str_to_idl_value(inner) })
        }
        _ if s.starts_with("Option<") => {
            let inner = s
                .strip_prefix("Option<")
                .and_then(|s| s.strip_suffix('>'))
                .expect("syn-generated type string has balanced angle brackets");
            json!({ "option": type_str_to_idl_value(inner) })
        }
        _ if s.starts_with("Box<") => {
            let inner = s
                .strip_prefix("Box<")
                .and_then(|s| s.strip_suffix('>'))
                .expect("syn-generated type string has balanced angle brackets");
            type_str_to_idl_value(inner)
        }
        other => json!({ "defined": { "name": strip_type_generics(other) } }),
    }
}

/// Drop the `<...>` suffix on a user-defined type name.
///
/// `MixedArgs<'_>` / `MixedArgs<'info>` → `MixedArgs`.
/// `PodVec<PodU64, 16>` → `PodVec`.
///
/// The IDL spec's `IdlType::Defined { name, generics }` models generic
/// references structurally (spec:284+), but the v2 derive doesn't yet emit
/// the `generics` payload. Meanwhile, `#[derive(IdlType)]` registers types
/// under the bare ident (no `<...>`), so leaking the generic suffix into
/// the reference produces a `{"defined":{"name":"Foo<'_>"}}` that never
/// resolves against the `types[]` entry named `"Foo"`. Strip here so the
/// two sides agree — downstream TS clients used to patch this at runtime
/// (`tests/shared.ts::loadIdl`).
///
/// Limitation: multiple instantiations of the same generic type
/// (`PodVec<PodU64, 16>` + `PodVec<PodU32, 8>`) collapse to the same
/// `"PodVec"` defined name. Fine for today's single-instantiation
/// programs; a proper fix needs full generics emission.
fn strip_type_generics(name: &str) -> &str {
    match name.find('<') {
        Some(idx) => &name[..idx],
        None => name,
    }
}

/// Strip a leading `&` reference and any `'lifetime` annotation from a type
/// string, so `&'a [u64]` → `[u64]`. Leaves nested types alone.
fn strip_ref_and_lifetime(s: &str) -> String {
    let s = s.trim();
    let s = s.strip_prefix('&').unwrap_or(s).trim_start();
    let s = if let Some(rest) = s.strip_prefix('\'') {
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
    /// Stringified RHS of `#[account(address = <expr>)]`. When `Some`,
    /// takes precedence over `IdlAccountType::__IDL_ADDRESS` at emission.
    /// Holds either a resolvable constant path / const-fn call (which the
    /// Anchor CLI can turn into a base58 pubkey) or a dotted field path
    /// like `data.authority` that clients walk at resolution time.
    pub address_override: Option<&'a str>,
    /// Set when this field is a `Nested<Inner>`, carrying the inner
    /// struct type. The emission splices the inner struct's own
    /// `__idl_accounts()` into the outer array instead of producing a
    /// single account entry for the `Nested` wrapper, so the IDL's
    /// `accounts[]` list flattens the nested block in source order —
    /// matching how the runtime actually consumes accounts.
    pub nested_inner_ty: Option<&'a Type>,
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
            // `Nested<Inner>` flattens at IDL time. Ask the inner struct
            // for its own `__idl_accounts()` string, strip the outer
            // `[` / `]`, and splice the element sequence in place. The
            // outer's join-with-`,` loop then produces a single flat
            // array in source order.
            if let Some(inner) = f.nested_inner_ty {
                return quote! {
                    {
                        let __inner = <#inner>::__idl_accounts();
                        // Strip the bracketing `[`/`]` produced by the
                        // inner emission. Use char-indexed slicing
                        // rather than `trim_matches`, which would also
                        // eat balanced brackets from inside string
                        // literals (there are none today, but the
                        // narrow form is future-proof).
                        let __bytes = __inner.as_bytes();
                        if __bytes.len() >= 2
                            && __bytes[0] == b'['
                            && __bytes[__bytes.len() - 1] == b']'
                        {
                            __inner[1..__inner.len() - 1].to_string()
                        } else {
                            __inner
                        }
                    }
                };
            }
            let name = f.name;
            let writable_json = if f.writable { ",\"writable\":true" } else { "" };
            let optional_json = if f.is_optional {
                ",\"optional\":true"
            } else {
                ""
            };
            let relations_json = if f.relations.is_empty() {
                String::new()
            } else {
                let list: Vec<String> = f.relations.iter().map(|r| format!("\"{r}\"")).collect();
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
            // `#[account(address = <expr>)]` override, pre-formatted as a
            // JSON fragment. When set, takes precedence over the wrapper
            // type's `__IDL_ADDRESS` — emitted at macro time so no runtime
            // branch is needed to pick one.
            let address_override_json = f
                .address_override
                .map(|s| format!(",\"address\":\"{s}\""))
                .unwrap_or_default();
            let init_signer = f.init_signer;
            if let Some(ty) = f.field_ty {
                let addr_json_expr = if f.address_override.is_some() {
                    quote! {
                        let __addr_json: anchor_lang_v2::__alloc::string::String =
                            anchor_lang_v2::__alloc::string::String::from(#address_override_json);
                    }
                } else {
                    quote! {
                        let __addr = <#ty as anchor_lang_v2::IdlAccountType>::__IDL_ADDRESS;
                        let __addr_json: anchor_lang_v2::__alloc::string::String = match __addr {
                            Some(a) => anchor_lang_v2::__alloc::format!(",\"address\":\"{}\"", a),
                            None => anchor_lang_v2::__alloc::string::String::new(),
                        };
                    }
                };
                quote! {
                    {
                        // Trait-const OR compile-time init_signer flag.
                        // Kept separate so a Signer + init-without-seeds
                        // combo still renders exactly one `"signer":true`.
                        let __signer = <#ty as anchor_lang_v2::IdlAccountType>::__IDL_IS_SIGNER
                            || #init_signer;
                        let __signer_json: &str = if __signer { ",\"signer\":true" } else { "" };
                        #addr_json_expr
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
                        "{{\"name\":\"{}\"{}{}{}{}{}{}{}}}",
                        #name,
                        #writable_json,
                        #signer_json,
                        #address_override_json,
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
                // A `Nested<Inner>` whose inner has zero fields contributes
                // an empty part — skip it so we don't emit `,,` or a leading
                // comma.
                if __p.is_empty() { continue; }
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
    let arr: Vec<Value> = args
        .iter()
        .map(|(name, ty)| {
            json!({
                "name": name.to_string(),
                "type": rust_type_to_idl_value(ty),
            })
        })
        .collect();
    Value::Array(arr).to_string()
}

/// Build discriminator JSON array from hash bytes.
pub fn disc_json(disc_bytes: &[u8]) -> String {
    disc_json_value(disc_bytes).to_string()
}

fn disc_json_value(disc_bytes: &[u8]) -> Value {
    Value::Array(disc_bytes.iter().map(|b| json!(*b)).collect())
}

/// Borsh / bytemuck mode tag passed down from the `#[account]` / `#[event]`
/// call sites. The spec (`idl/spec/src/lib.rs:180-216`) models this as the
/// pair `(IdlSerialization, Option<IdlRepr>)`, but the two fields are tightly
/// coupled — bytemuck always pairs with `repr(C)` in our codegen, borsh
/// carries no repr — so we collapse them into a single enum and expand both
/// fields at emission time.
///
/// Default `#[event]` (wincode under the hood) is also tagged `Borsh` here:
/// the wire format is borsh-compatible via `BORSH_CONFIG`, so off-chain
/// consumers decode it as borsh.
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
    let mut out = build_type_header(name, disc, docs, kind);
    let field_values: Vec<Value> = fields.iter().map(named_field_value).collect();
    out.insert(
        "type".into(),
        json!({ "kind": "struct", "fields": field_values }),
    );
    Value::Object(out).to_string()
}

/// Build IDL type definition JSON from enum variants. Matches the spec's
/// `IdlTypeDefTy::Enum { variants }` shape (idl/spec/src/lib.rs:237-248).
/// Each variant is emitted as `{"name": ..., "fields"?: ...}` where `fields`
/// is either a named-field object array (struct-like variants), a tuple of
/// types (tuple-like variants), or omitted entirely (unit variants) —
/// consistent with `IdlDefinedFields`'s `#[serde(untagged)]` shape.
///
/// Only `TypeKind::Borsh` is really meaningful for enums today — bytemuck
/// enums are rare. We accept the same `kind` parameter for shape symmetry
/// with `build_type_json`.
pub fn build_enum_type_json(
    name: &str,
    disc: &[u8],
    docs: &[String],
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
    kind: TypeKind,
) -> String {
    let mut out = build_type_header(name, disc, docs, kind);
    let variant_values: Vec<Value> = variants
        .iter()
        .map(|v| {
            let mut obj = serde_json::Map::new();
            obj.insert("name".into(), Value::String(v.ident.to_string()));
            match &v.fields {
                syn::Fields::Unit => {}
                syn::Fields::Named(named) => {
                    let fields: Vec<Value> = named.named.iter().map(named_field_value).collect();
                    obj.insert("fields".into(), Value::Array(fields));
                }
                syn::Fields::Unnamed(unnamed) => {
                    let tys: Vec<Value> = unnamed
                        .unnamed
                        .iter()
                        .map(|f| rust_type_to_idl_value(&f.ty))
                        .collect();
                    obj.insert("fields".into(), Value::Array(tys));
                }
            }
            Value::Object(obj)
        })
        .collect();
    out.insert(
        "type".into(),
        json!({ "kind": "enum", "variants": variant_values }),
    );
    Value::Object(out).to_string()
}

/// Shared header construction for struct and enum type definitions. Emits
/// `name`, `discriminator`, optional `docs`, and the `serialization` / `repr`
/// pair derived from `kind`. The caller appends the `type` object matching
/// `IdlTypeDefTy::{Struct, Enum, Type}`.
fn build_type_header(
    name: &str,
    disc: &[u8],
    docs: &[String],
    kind: TypeKind,
) -> serde_json::Map<String, Value> {
    let mut out = serde_json::Map::new();
    out.insert("name".into(), Value::String(name.to_owned()));
    out.insert("discriminator".into(), disc_json_value(disc));
    if !docs.is_empty() {
        out.insert("docs".into(), docs_value(docs));
    }
    match kind {
        TypeKind::Borsh => {}
        TypeKind::BytemuckRepr => {
            out.insert("serialization".into(), Value::String("bytemuck".into()));
            out.insert("repr".into(), json!({ "kind": "c" }));
        }
    }
    out
}

/// Build a named `IdlField` value — `{name, type, docs?}` — for a single
/// `syn::Field`. Used by both struct field and enum-variant struct-field
/// emission.
fn named_field_value(f: &syn::Field) -> Value {
    let fname = f
        .ident
        .as_ref()
        .expect("named fields always have idents")
        .to_string();
    let mut obj = serde_json::Map::new();
    obj.insert("name".into(), Value::String(fname));
    let field_docs = extract_doc_lines(&f.attrs);
    if !field_docs.is_empty() {
        obj.insert("docs".into(), docs_value(&field_docs));
    }
    obj.insert("type".into(), rust_type_to_idl_value(&f.ty));
    Value::Object(obj)
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
                        return Some(v.strip_prefix(' ').map(str::to_owned).unwrap_or(v));
                    }
                }
            }
            None
        })
        .collect()
}

/// Serialize a list of doc lines into a JSON array string.
pub fn docs_to_json_array(docs: &[String]) -> String {
    docs_value(docs).to_string()
}

fn docs_value(docs: &[String]) -> Value {
    Value::Array(docs.iter().map(|d| Value::String(d.clone())).collect())
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
///   → `{"kind":"account","path":"user"}`
/// - instruction arg ref (`nonce` bare ident,
///   `nonce.to_le_bytes()`, `nonce.as_ref()`)
///   with `nonce` in `ix_arg_names`
///   → `{"kind":"arg","path":"nonce"}`
/// - anything else (e.g. `Some::Path::call()`) → `const` with empty value
///   + eprintln warning so the anchor build CLI surfaces it.
pub fn classify_seed(expr: &Expr, field_names: &[String], ix_arg_names: &[String]) -> String {
    classify_seed_value(expr, field_names, ix_arg_names).to_string()
}

fn classify_seed_value(expr: &Expr, field_names: &[String], ix_arg_names: &[String]) -> Value {
    // Peel `&<inner>` wrappers — they're common in seed expressions and
    // always transparent to classification.
    let mut cur = expr;
    while let Expr::Reference(r) = cur {
        cur = &r.expr;
    }

    if let Expr::Lit(lit) = cur {
        match &lit.lit {
            Lit::ByteStr(bs) => return const_seed_value(&bs.value()),
            Lit::Str(s) => return const_seed_value(s.value().as_bytes()),
            Lit::Byte(b) => return const_seed_value(&[b.value()]),
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
            return const_seed_value(&b);
        }
    }

    // Bare ident — field ref wins, then ix arg.
    if let Expr::Path(ep) = cur {
        if ep.qself.is_none() && ep.path.segments.len() == 1 && ep.path.leading_colon.is_none() {
            let seg = &ep.path.segments[0];
            if seg.arguments.is_empty() {
                let name = seg.ident.to_string();
                if field_names.contains(&name) {
                    return account_seed_value(&name);
                }
                if ix_arg_names.contains(&name) {
                    return arg_seed_value(&name);
                }
            }
        }
    }

    // Method-call / field-access chains: walk to the receiver's bare
    // ident. Handles `foo.bar()`, `foo.bar.baz`, `foo.key().as_ref()`,
    // `foo.to_le_bytes()`, `foo[0]`, `(foo)`, etc.
    if let Some(root) = receiver_root_ident(cur) {
        if field_names.contains(&root) {
            return account_seed_value(&root);
        }
        if ix_arg_names.contains(&root) {
            return arg_seed_value(&root);
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
                return const_seed_value(s.value().as_bytes());
            }
        }
    }

    // Zero-arg path-call like `AssociatedToken::id()` or `System::id()`.
    // These show up in `seeds::program = <Marker>::id()` overrides (ATA
    // derivation is the canonical case). The `Id` trait's `IDL_ADDRESS`
    // already declares each marker's pubkey at the type level; we mirror
    // the mapping here and emit the base58-decoded bytes as a const seed
    // so clients derive the PDA against the correct program override
    // rather than the empty-bytes fallback.
    //
    // Only the five well-known markers ship in `lang-v2/src/programs.rs`;
    // custom user markers still fall through to the warn path.
    if let Expr::Call(call) = cur {
        if call.args.is_empty() {
            if let Expr::Path(ep) = &*call.func {
                let segs = &ep.path.segments;
                if segs.len() >= 2 && segs[segs.len() - 1].ident == "id" {
                    let marker = segs[segs.len() - 2].ident.to_string();
                    let addr_b58: Option<&str> = match marker.as_str() {
                        "System" => Some("11111111111111111111111111111111"),
                        "Token" => Some("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
                        "Token2022" => Some("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"),
                        "AssociatedToken" => Some("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"),
                        "Memo" => Some("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr"),
                        _ => None,
                    };
                    if let Some(b58) = addr_b58 {
                        if let Ok(bytes) = bs58::decode(b58).into_vec() {
                            return const_seed_value(&bytes);
                        }
                    }
                }
            }
        }
    }

    eprintln!(
        "anchor-v2 idl: unable to classify seed expression `{}`; emitting empty const",
        quote!(#expr)
    );
    const_seed_value(&[])
}

/// Walk down a method-call / field-access / index chain and return the
/// bare ident at its root, if any. `foo.key().as_ref()` → `foo`;
/// `foo.bar.baz` → `foo`; `foo[0]` → `foo`; `(foo)` → `foo`.
pub fn receiver_root_ident_str(expr: &Expr) -> Option<String> {
    receiver_root_ident(expr)
}

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

fn const_seed_value(bytes: &[u8]) -> Value {
    json!({ "kind": "const", "value": bytes })
}

fn account_seed_value(path: &str) -> Value {
    json!({ "kind": "account", "path": path })
}

fn arg_seed_value(path: &str) -> Value {
    json!({ "kind": "arg", "path": path })
}

/// Assemble the `pda: {...}` object body from a field's classified seeds
/// plus optional program override. Returns the JSON object (without the
/// leading `,"pda":` — that's spliced by `build_accounts_emission`).
pub fn pda_object_json(seeds: &[String], program: Option<&String>) -> String {
    // Seed strings come in pre-serialized (via `classify_seed`) so we
    // re-parse them back to `Value`s for structured composition. The
    // parse can't fail because `classify_seed` controls the producer.
    let seeds_arr: Vec<Value> = seeds
        .iter()
        .map(|s| serde_json::from_str(s).expect("classify_seed emits valid JSON"))
        .collect();
    let mut obj = serde_json::Map::new();
    obj.insert("seeds".into(), Value::Array(seeds_arr));
    if let Some(p) = program {
        // The program override comes from `seeds::program = <expr>` and is
        // also a pre-serialized JSON value (an address or a defined ref).
        let program_val: Value = serde_json::from_str(p).expect("program seed value is valid JSON");
        obj.insert("program".into(), program_val);
    }
    Value::Object(obj).to_string()
}
