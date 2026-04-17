extern crate proc_macro;

mod access_control;
mod constant;
mod error_code;
mod idl;
mod init_space;
mod parse;
mod pda;

use {
    proc_macro::TokenStream,
    proc_macro2::TokenStream as TokenStream2,
    quote::quote,
    syn::{
        parse_macro_input, spanned::Spanned, Data, DeriveInput, Fields, FnArg, Ident, ItemMod, Pat,
        Type,
    },
};

// ---------------------------------------------------------------------------
// #[derive(Accounts)]
// ---------------------------------------------------------------------------

#[proc_macro_derive(Accounts, attributes(account, instruction))]
pub fn derive_accounts(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(impl_accounts(&input))
}

/// Returns true if `ty` needs the `'ix` lifetime injected when used as an
/// instruction arg. This is the case for top-level references (`&[u8]`, `&T`)
/// and for path types carrying lifetime generic args (`CreateArgs<'_>`,
/// `Option<&[u8]>`, etc.).
fn needs_ix_lifetime(ty: &Type) -> bool {
    match ty {
        Type::Reference(_) => true,
        Type::Path(tp) => tp.path.segments.iter().any(|seg| {
            if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                ab.args.iter().any(|arg| match arg {
                    syn::GenericArgument::Lifetime(_) => true,
                    syn::GenericArgument::Type(inner) => needs_ix_lifetime(inner),
                    _ => false,
                })
            } else {
                false
            }
        }),
        _ => false,
    }
}

/// Recursively rewrites any elided or named lifetimes in `ty` to `ix`.
///
/// - `&[T]` / `&T` with elided lifetime → `&'ix [T]` / `&'ix T`
///   (explicit lifetimes on references are preserved — a handler asking for
///   `&'static [u8]` still gets `'static`)
/// - `Foo<'_>`, `Foo<'a, ...>` → `Foo<'ix, ...>` (every lifetime arg in the
///   path gets rewritten; users can't introduce named lifetimes at the
///   handler scope anyway)
/// - Nested types are walked (`Option<&[u8]>`, `Result<Args<'_>, E>`, ...)
///
/// This lets a handler fn take a borrowed struct arg like
/// `args: MyArgs<'_>` and have the generated `__Args` struct bind the
/// lifetime correctly.
fn with_ix_lifetime(ty: &Type, ix: &syn::Lifetime) -> Type {
    match ty {
        Type::Reference(tr) => {
            let mut new_tr = tr.clone();
            let is_elided = new_tr
                .lifetime
                .as_ref()
                .map(|l| l.ident == "_")
                .unwrap_or(true);
            if is_elided {
                new_tr.lifetime = Some(ix.clone());
            }
            new_tr.elem = Box::new(with_ix_lifetime(&new_tr.elem, ix));
            Type::Reference(new_tr)
        }
        Type::Path(tp) => {
            let mut new_tp = tp.clone();
            for seg in new_tp.path.segments.iter_mut() {
                if let syn::PathArguments::AngleBracketed(ab) = &mut seg.arguments {
                    for arg in ab.args.iter_mut() {
                        match arg {
                            syn::GenericArgument::Lifetime(lt) => {
                                *lt = ix.clone();
                            }
                            syn::GenericArgument::Type(inner) => {
                                *inner = with_ix_lifetime(inner, ix);
                            }
                            _ => {}
                        }
                    }
                }
            }
            Type::Path(new_tp)
        }
        _ => ty.clone(),
    }
}

struct ArgsDeser {
    deser: TokenStream2,
    arg_types: Vec<Type>,
    has_refs: bool,
}

/// Build the `#[derive(SchemaRead)] struct + deserialize` block for a list of
/// `(name, type)` argument pairs. Used by both `#[instruction(...)]` in
/// `impl_accounts` and handler extra-args in `impl_program`.
///
/// `inline_error`: when `true`, deser failure returns a `u64` directly (handler
/// wrapper context); when `false`, it returns `Err(...)` (try_accounts context).
fn emit_args_deser(args: &[(&Ident, &Type)], struct_name: &str, inline_error: bool) -> ArgsDeser {
    let ix_lifetime: syn::Lifetime = syn::parse_quote!('ix);
    let arg_types: Vec<Type> = args
        .iter()
        .map(|(_, t)| with_ix_lifetime(t, &ix_lifetime))
        .collect();
    let has_refs = args.iter().any(|(_, t)| needs_ix_lifetime(t));
    let (lt_decl, lt_use) = if has_refs {
        (quote! { <'ix> }, quote! { <'_> })
    } else {
        (quote! {}, quote! {})
    };

    let names: Vec<_> = args.iter().map(|(n, _)| *n).collect();
    let struct_ident = Ident::new(struct_name, proc_macro2::Span::call_site());

    let deser = if args.is_empty() {
        quote! {}
    } else {
        let error_handling = if inline_error {
            quote! {
                match anchor_lang_v2::wincode::deserialize(__ix_data) {
                    Ok(__v) => __v,
                    Err(_) => return {
                        let __e: anchor_lang_v2::Error =
                            anchor_lang_v2::ErrorCode::InstructionDidNotDeserialize.into();
                        __e.into()
                    },
                }
            }
        } else {
            quote! {
                anchor_lang_v2::wincode::deserialize(__ix_data)
                    .map_err(|_| anchor_lang_v2::ErrorCode::InstructionDidNotDeserialize)?
            }
        };
        quote! {
            #[derive(anchor_lang_v2::wincode::SchemaRead)]
            struct #struct_ident #lt_decl { #(#names: #arg_types,)* }
            let __args: #struct_ident #lt_use = #error_handling;
            #(let #names = __args.#names;)*
        }
    };

    ArgsDeser {
        deser,
        arg_types,
        has_refs,
    }
}

/// Parse `#[instruction(name: Type, ...)]` from struct-level attributes.
/// Returns a list of (name, type) pairs.
fn parse_instruction_attrs(attrs: &[syn::Attribute]) -> Vec<(Ident, Type)> {
    let mut result = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("instruction") {
            continue;
        }
        let _ = attr.parse_args_with(|input: syn::parse::ParseStream| {
            while !input.is_empty() {
                let name: Ident = input.parse()?;
                input.parse::<syn::Token![:]>()?;
                let ty: Type = input.parse()?;
                result.push((name, ty));
                if !input.is_empty() {
                    input.parse::<syn::Token![,]>()?;
                }
            }
            Ok(())
        });
    }
    result
}

fn impl_accounts(input: &DeriveInput) -> TokenStream2 {
    let name = &input.ident;
    let bumps_name = syn::Ident::new(&format!("{name}Bumps"), name.span());

    // Bail with a properly-spanned diagnostic on unsupported shapes.
    let named_fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => named,
            _ => {
                return syn::Error::new(name.span(), "`Accounts` derive only supports named fields")
                    .to_compile_error()
            }
        },
        _ => {
            return syn::Error::new(name.span(), "`Accounts` derive only supports structs")
                .to_compile_error()
        }
    };

    // Collect field names first so we can rewrite bare-ident seed expressions.
    let raw_field_names: Vec<String> = named_fields
        .named
        .iter()
        .filter_map(|f| f.ident.as_ref().map(|i| i.to_string()))
        .collect();

    if named_fields.named.len() > 255 {
        return syn::Error::new(
            name.span(),
            "`Accounts` derive supports at most 255 fields",
        )
        .to_compile_error();
    }

    // Parse #[instruction(arg: Type, ...)] — consumed both for the
    // runtime deser block AND for PDA seed classification: seeds that
    // reference an ix arg resolve to `IdlSeed::Arg{path}`.
    let ix_args = parse_instruction_attrs(&input.attrs);
    let ix_arg_names: Vec<String> = ix_args.iter().map(|(n, _)| n.to_string()).collect();

    // Compute the views-slice offset for each field. Direct fields occupy 1
    // slot; `Nested<Inner>` fields occupy `Inner::HEADER_SIZE` slots. Each
    // offset is a const expression resolved at monomorphization time.
    let mut offset_exprs: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut current_offset: proc_macro2::TokenStream = quote::quote! { 0usize };
    for f in named_fields.named.iter() {
        offset_exprs.push(current_offset.clone());
        if let Some(inner_ty) = parse::extract_nested_inner_type(&f.ty) {
            current_offset = quote::quote! {
                #current_offset + <#inner_ty as anchor_lang_v2::TryAccounts>::HEADER_SIZE
            };
        } else {
            current_offset = quote::quote! { #current_offset + 1 };
        }
    }

    let fields: Vec<parse::AccountField> = named_fields
        .named
        .iter()
        .zip(offset_exprs.into_iter())
        .map(|(f, offset)| parse::parse_field(f, &raw_field_names, offset, &ix_arg_names))
        .collect();

    let field_names: Vec<_> = fields.iter().map(|f| &f.name).collect();
    let loads: Vec<_> = fields.iter().map(|f| &f.load).collect();
    let constraints: Vec<_> = fields.iter().flat_map(|f| &f.constraints).collect();
    let exits: Vec<_> = fields.iter().filter_map(|f| f.exit.as_ref()).collect();
    // Bumps fields. Optional accounts get `Option<u8>` so the default
    // (`None`) maps cleanly to the sentinel-`None` load path; the seeds
    // check assigns `Some(bump)` only when the inner is `Some`. Mirrors
    // v1's `bumps.rs:36` Optional handling.
    let bump_fields: Vec<proc_macro2::TokenStream> = fields
        .iter()
        .filter(|f| f.has_bump)
        .map(|f| {
            let n = &f.name;
            if f.is_optional {
                quote! { #n: Option<u8> }
            } else {
                quote! { #n: u8 }
            }
        })
        .collect();

    // Compile-time sum for `<T as TryAccounts>::HEADER_SIZE`:
    //   - 1 per non-`Nested<_>` field (consumes one account view)
    //   - `<Inner as TryAccounts>::HEADER_SIZE` per `Nested<Inner>` field,
    //     which recursively expands at monomorphization time.
    // The direct-field count is a single literal so the emitted
    // const is short in the common (no-nested) case.
    let direct_count: usize = fields
        .iter()
        .filter(|f| !parse::is_nested_type(&f.ty))
        .count();
    let nested_inner_types: Vec<&syn::Type> = fields
        .iter()
        .filter_map(|f| parse::extract_nested_inner_type(&f.ty))
        .collect();
    let header_size_expr = if nested_inner_types.is_empty() {
        quote::quote! { #direct_count }
    } else {
        quote::quote! {
            #direct_count #(+ <#nested_inner_types as anchor_lang_v2::TryAccounts>::HEADER_SIZE)*
        }
    };

    // IDL collection — the accounts-JSON emission is a runtime function
    // (not a `&'static str` const) so it can read
    // `<FieldTy as IdlAccountType>::__IDL_IS_SIGNER / __IDL_ADDRESS` off
    // the wrapper type. Compile-time flags (writable, init_signer,
    // optional, relations) are baked directly into the format strings, so
    // the runtime only pays for trait dispatch + concatenation.
    let field_names_str: Vec<String> = fields.iter().map(|f| f.name.to_string()).collect();

    // Build the inverse has_one mapping: relations on field `X` lists every
    // sibling whose `has_one = X` chain targets X. Mirrors v1's
    // `get_relations` — relations live on the target, not the source.
    let relations_by_target: std::collections::HashMap<String, Vec<String>> = {
        let mut m: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for f in &fields {
            let src = f.name.to_string();
            for target in &f.idl_has_one {
                m.entry(target.clone()).or_default().push(src.clone());
            }
        }
        m
    };

    // Pre-build per-field pda JSON bodies so the emission site only has
    // to splice strings. Saves duplicating the seed-classification logic.
    let pda_jsons: Vec<Option<String>> = fields
        .iter()
        .map(|f| {
            f.idl_pda
                .as_ref()
                .map(|p| idl::pda_object_json(&p.seeds, p.program.as_ref()))
        })
        .collect();

    let accounts_fields: Vec<idl::AccountsJsonField<'_>> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let name: &str = &field_names_str[i];
            let relations: Vec<&str> = relations_by_target
                .get(name)
                .map(|v| v.iter().map(|s| s.as_str()).collect())
                .unwrap_or_default();
            idl::AccountsJsonField {
                name,
                writable: f.idl_writable,
                init_signer: f.idl_init_signer,
                is_optional: f.is_optional,
                relations,
                docs: &f.idl_docs,
                pda_json: pda_jsons[i].clone(),
                field_ty: &f.idl_field_ty,
                // `Nested<Inner>` flattens at IDL emission time by calling
                // into `Inner::__idl_accounts()`. Grab the inner `Type` so
                // the emitter can synthesize that call.
                nested_inner_ty: parse::extract_nested_inner_type(&f.ty),
            }
        })
        .collect();
    let idl_accounts_fn = idl::build_accounts_emission(&accounts_fields);
    // Only path-typed fields drive the IDL dep walk. `idl_field_ty` is
    // already the post-Option-unwrap base type (see `parse::parse_field`),
    // and is `None` for non-Path fields — filter those out so the emitted
    // trait call has a concrete type to dispatch on.
    let idl_field_tys: Vec<&syn::Type> = fields
        .iter()
        .filter_map(|f| f.idl_field_ty.as_ref())
        .collect();

    let ix_deser = if ix_args.is_empty() {
        quote! {}
    } else {
        let pairs: Vec<(&Ident, &Type)> = ix_args.iter().map(|(n, t)| (n, t)).collect();
        emit_args_deser(&pairs, "__IxArgs", false).deser
    };

    // Conditional bumps: empty → type alias, non-empty → struct
    let has_bumps = !bump_fields.is_empty();
    let bumps_def = if has_bumps {
        quote! {
            #[derive(Default, Clone)]
            pub struct #bumps_name { #(pub #bump_fields,)* }
        }
    } else {
        quote! { pub type #bumps_name = (); }
    };
    let bumps_init = if has_bumps {
        quote! { let mut __bumps = #bumps_name::default(); }
    } else {
        quote! { let __bumps = #bumps_name::default(); }
    };

    // --- Client-side struct for off-chain usage (tests, CPI, SDK) ---
    //
    // The struct only contains fields the user must provide (Signer, raw
    // accounts, etc.). Derivable fields (Program<T>, PDAs) are computed
    // inside `to_account_metas()` so the user never has to fill them.
    let client_mod_name = syn::Ident::new(
        &format!("__client_accounts_{}", name.to_string().to_lowercase()),
        name.span(),
    );

    // Classify each field as user-required or auto-derivable.
    #[derive(Clone)]
    enum FieldKind {
        Required,
        Program(proc_macro2::TokenStream),
        Pda {
            seed_exprs: Vec<proc_macro2::TokenStream>,
            deps: Vec<String>,
        },
    }

    let field_kinds: Vec<_> = fields
        .iter()
        .zip(named_fields.named.iter())
        .map(|(f, raw_field)| {
            let base_ty = match parse::extract_option_inner(&f.ty) {
                Some(inner) => inner,
                None => &f.ty,
            };
            let ty_name = parse::field_ty_str(base_ty);

            if ty_name == "Program" {
                if let Type::Path(tp) = base_ty {
                    if let Some(seg) = tp.path.segments.last() {
                        if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                            if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                                return (f, FieldKind::Program(quote! {
                                    <#inner as anchor_lang_v2::Id>::id()
                                }));
                            }
                        }
                    }
                }
            }

            let attrs = parse::parse_account_attrs(&raw_field.attrs);
            if let Some(ref seeds) = attrs.seeds {
                if attrs.seeds_program.is_none() {
                    let mut seed_exprs = Vec::new();
                    let mut deps = Vec::new();
                    let mut all_derivable = true;

                    for seed in seeds {
                        if let Some(bytes) = pda::seed_as_const_bytes(seed) {
                            let lits: Vec<_> = bytes.iter().map(|b| quote! { #b }).collect();
                            seed_exprs.push(quote! { &[#(#lits),*] as &[u8] });
                        } else if let Some(ref root) = idl::receiver_root_ident_str(seed) {
                            if raw_field_names.contains(root) {
                                let ident = syn::Ident::new(
                                    root,
                                    proc_macro2::Span::call_site(),
                                );
                                seed_exprs.push(quote! { #ident.as_ref() });
                                deps.push(root.clone());
                            } else {
                                all_derivable = false;
                                break;
                            }
                        } else {
                            all_derivable = false;
                            break;
                        }
                    }

                    if all_derivable {
                        return (f, FieldKind::Pda { seed_exprs, deps });
                    }
                }
            }

            (f, FieldKind::Required)
        })
        .collect();

    // Client struct fields: only the ones the user must provide.
    let client_fields: Vec<_> = field_kinds
        .iter()
        .filter(|(_, kind)| matches!(kind, FieldKind::Required))
        .map(|(f, _)| {
            let fname = &f.name;
            if f.is_optional {
                quote! { pub #fname: Option<anchor_lang_v2::Address> }
            } else {
                quote! { pub #fname: anchor_lang_v2::Address }
            }
        })
        .collect();

    // Topo-sort derivable fields: programs first, then PDAs by dependency order.
    let required_names: std::collections::HashSet<String> = field_kinds
        .iter()
        .filter(|(_, kind)| matches!(kind, FieldKind::Required))
        .map(|(f, _)| f.name.to_string())
        .collect();

    let mut derive_order: Vec<usize> = Vec::new();
    let mut resolved = required_names.clone();
    for (i, (f, kind)) in field_kinds.iter().enumerate() {
        if matches!(kind, FieldKind::Program(_)) {
            derive_order.push(i);
            resolved.insert(f.name.to_string());
        }
    }
    let mut progress = true;
    while progress {
        progress = false;
        for (i, (f, kind)) in field_kinds.iter().enumerate() {
            let name_str = f.name.to_string();
            if resolved.contains(&name_str) {
                continue;
            }
            if let FieldKind::Pda { deps, .. } = kind {
                if deps.iter().all(|dep| resolved.contains(dep)) {
                    derive_order.push(i);
                    resolved.insert(name_str);
                    progress = true;
                }
            }
        }
    }

    // Inside to_account_metas: copy required fields to locals, then derive the rest.
    let required_locals: Vec<_> = field_kinds
        .iter()
        .filter(|(_, kind)| matches!(kind, FieldKind::Required))
        .map(|(f, _)| {
            let fname = &f.name;
            if f.is_optional {
                quote! { let #fname = self.#fname; }
            } else {
                quote! { let #fname = self.#fname; }
            }
        })
        .collect();

    let derive_stmts: Vec<_> = derive_order
        .iter()
        .filter_map(|&i| {
            let (f, kind) = &field_kinds[i];
            let ident = &f.name;
            // When the field is `Option<Program<T>>` / `Option<...seeds...>`,
            // the downstream `client_meta_entries` match expects the local
            // binding to be `Option<Address>`, not a bare `Address`. Wrap
            // the derived value in `Some(...)` in that case so the match
            // arms type-check.
            match kind {
                FieldKind::Program(expr) => {
                    let init = if f.is_optional {
                        quote! { Some(#expr) }
                    } else {
                        quote! { #expr }
                    };
                    Some(quote! { let #ident = #init; })
                }
                FieldKind::Pda { seed_exprs, .. } => {
                    if f.is_optional {
                        Some(quote! {
                            let #ident = {
                                let (__addr, _) = anchor_lang_v2::find_program_address(
                                    &[#(#seed_exprs),*], &crate::ID,
                                );
                                Some(__addr)
                            };
                        })
                    } else {
                        Some(quote! {
                            let (#ident, _) = anchor_lang_v2::find_program_address(
                                &[#(#seed_exprs),*], &crate::ID,
                            );
                        })
                    }
                }
                _ => None,
            }
        })
        .collect();

    // Build AccountMeta entries in original field order, using bare idents.
    let client_meta_entries: Vec<_> = field_kinds
        .iter()
        .map(|(field, _kind)| {
            let writable = field.idl_writable;
            let is_signer_ty = parse::field_ty_str(match parse::extract_option_inner(&field.ty) {
                Some(inner) => inner,
                None => &field.ty,
            }) == "Signer";
            let signer = is_signer_ty || field.idl_init_signer;
            let field_ident = &field.name;
            if field.is_optional {
                quote! {
                    match #field_ident {
                        Some(__addr) => anchor_lang_v2::AccountMeta {
                            pubkey: __addr,
                            is_writable: #writable,
                            is_signer: #signer,
                        },
                        None => anchor_lang_v2::AccountMeta {
                            pubkey: crate::ID,
                            is_writable: false,
                            is_signer: false,
                        },
                    }
                }
            } else {
                quote! {
                    anchor_lang_v2::AccountMeta {
                        pubkey: #field_ident,
                        is_writable: #writable,
                        is_signer: #signer,
                    }
                }
            }
        })
        .collect();

    // PDA finder functions for seed-bearing fields.
    let pda_fns: Vec<TokenStream2> = fields
        .iter()
        .filter_map(|f| {
            let attrs = parse::parse_account_attrs(
                &named_fields
                    .named
                    .iter()
                    .find(|nf| nf.ident.as_ref() == Some(&f.name))?
                    .attrs,
            );
            let seeds = attrs.seeds.as_ref()?;

            let field_name = &f.name;
            let fn_name = syn::Ident::new(
                &format!("find_{}_address", field_name),
                field_name.span(),
            );

            let mut params: Vec<TokenStream2> = Vec::new();
            let mut seed_exprs: Vec<TokenStream2> = Vec::new();
            let mut seen_params = std::collections::HashSet::new();

            for seed_expr in seeds {
                let root = idl::receiver_root_ident_str(seed_expr);
                if let Some(ref root_name) = root {
                    if raw_field_names.contains(root_name) {
                        let param_ident = syn::Ident::new(root_name, proc_macro2::Span::call_site());
                        if seen_params.insert(root_name.clone()) {
                            params.push(quote! { #param_ident: &anchor_lang_v2::Address });
                        }
                        seed_exprs.push(quote! { #param_ident.as_ref() });
                        continue;
                    }
                    if ix_arg_names.contains(root_name) {
                        let param_ident = syn::Ident::new(root_name, proc_macro2::Span::call_site());
                        if seen_params.insert(root_name.clone()) {
                            params.push(quote! { #param_ident: &[u8] });
                        }
                        seed_exprs.push(quote! { #param_ident });
                        continue;
                    }
                }
                if let Some(bytes) = pda::seed_as_const_bytes(seed_expr) {
                    let byte_lits: Vec<_> = bytes.iter().map(|b| quote! { #b }).collect();
                    seed_exprs.push(quote! { &[#(#byte_lits),*] });
                } else {
                    return None;
                }
            }

            Some(quote! {
                pub fn #fn_name(#(#params),*) -> (anchor_lang_v2::Address, u8) {
                    anchor_lang_v2::find_program_address(
                        &[#(#seed_exprs),*],
                        &crate::ID,
                    )
                }
            })
        })
        .collect();

    // Full struct: all fields as Address, user fills every one manually.
    let all_client_fields: Vec<_> = fields
        .iter()
        .map(|f| {
            let fname = &f.name;
            if f.is_optional {
                quote! { pub #fname: Option<anchor_lang_v2::Address> }
            } else {
                quote! { pub #fname: anchor_lang_v2::Address }
            }
        })
        .collect();

    // Full struct to_account_metas: straightforward self.field access.
    let full_meta_entries: Vec<_> = fields
        .iter()
        .map(|field| {
            let writable = field.idl_writable;
            let is_signer_ty = parse::field_ty_str(match parse::extract_option_inner(&field.ty) {
                Some(inner) => inner,
                None => &field.ty,
            }) == "Signer";
            let signer = is_signer_ty || field.idl_init_signer;
            let field_ident = &field.name;
            if field.is_optional {
                quote! {
                    match self.#field_ident {
                        Some(__addr) => anchor_lang_v2::AccountMeta {
                            pubkey: __addr,
                            is_writable: #writable,
                            is_signer: #signer,
                        },
                        None => anchor_lang_v2::AccountMeta {
                            pubkey: crate::ID,
                            is_writable: false,
                            is_signer: false,
                        },
                    }
                }
            } else {
                quote! {
                    anchor_lang_v2::AccountMeta {
                        pubkey: self.#field_ident,
                        is_writable: #writable,
                        is_signer: #signer,
                    }
                }
            }
        })
        .collect();

    let resolved_name = syn::Ident::new(
        &format!("{name}Resolved"),
        name.span(),
    );

    let resolved_struct = quote! {
        pub struct #resolved_name {
            #(#client_fields,)*
        }
        impl anchor_lang_v2::ToAccountMetas for #resolved_name {
            fn to_account_metas(&self, _is_signer: Option<bool>) -> alloc::vec::Vec<anchor_lang_v2::AccountMeta> {
                #(#required_locals)*
                #(#derive_stmts)*
                alloc::vec![#(#client_meta_entries),*]
            }
        }
    };

    quote! {
        pub mod #client_mod_name {
            extern crate alloc;
            use super::*;
            pub struct #name {
                #(#all_client_fields,)*
            }
            impl anchor_lang_v2::ToAccountMetas for #name {
                fn to_account_metas(&self, _is_signer: Option<bool>) -> alloc::vec::Vec<anchor_lang_v2::AccountMeta> {
                    alloc::vec![#(#full_meta_entries),*]
                }
            }
            #resolved_struct
            impl #name {
                #(#pda_fns)*
            }
        }

        #bumps_def

        impl anchor_lang_v2::Bumps for #name {
            type Bumps = #bumps_name;
        }

        impl anchor_lang_v2::TryAccounts for #name {
            const HEADER_SIZE: usize = #header_size_expr;

            #[inline]
            fn try_accounts(
                __program_id: &anchor_lang_v2::Address,
                __views: &[anchor_lang_v2::AccountView],
                __duplicates: &anchor_lang_v2::AccountBitvec,
                __base_offset: usize,
                __ix_data: &[u8],
            ) -> anchor_lang_v2::Result<(Self, #bumps_name)> {
                #ix_deser
                #bumps_init
                #(#loads)*
                #(#constraints)*
                Ok((Self { #(#field_names),* }, __bumps))
            }

            //
            #[inline(always)]
            fn exit_accounts(&mut self) -> anchor_lang_v2::Result<()> {
                #(#exits)*
                Ok(())
            }
        }

        #[cfg(feature = "idl-build")]
        impl #name {
            // Runtime-assembled accounts JSON: reads per-wrapper signer /
            // address trait consts, splices in compile-time flags.
            #idl_accounts_fn

            /// Walks each account field's `IdlAccountType::__register_idl_deps`
            /// so nested user-defined types (plain `#[derive(IdlType)]`
            /// structs, `#[account]` data types, `#[event]` payload types,
            /// etc.) transitively register into the IDL's `types[]` array.
            pub fn __idl_register_deps(
                types: &mut anchor_lang_v2::__alloc::vec::Vec<&'static str>,
            ) {
                #(
                    <#idl_field_tys as anchor_lang_v2::IdlAccountType>::__register_idl_deps(types);
                )*
            }
        }
    }
}

// ---------------------------------------------------------------------------
// #[account]
// ---------------------------------------------------------------------------

#[proc_macro_attribute]
pub fn account(attr: TokenStream, item: TokenStream) -> TokenStream {
    let is_borsh = match parse_account_mode(attr) {
        Ok(b) => b,
        Err(err) => return err.to_compile_error().into(),
    };
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();
    let vis = &input.vis;
    let attrs = &input.attrs;
    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => {
            return syn::Error::new(name.span(), "`#[account]` only supports structs")
                .to_compile_error()
                .into()
        }
    };

    use sha2::Digest;
    let hash = sha2::Sha256::digest(format!("account:{name_str}").as_bytes());
    let disc_bytes = &hash[..8];
    let disc_literals: Vec<_> = disc_bytes.iter().map(|b| quote! { #b }).collect();

    let struct_docs = idl::extract_doc_lines(attrs);
    // `#[account]` has two modes: default zero-copy (Pod + repr(C)) and opt-in
    // borsh (`#[account(borsh)]`). The mode propagates into the IDL type
    // definition's `serialization` / `repr` fields (spec:180-216) so
    // downstream codegen knows which wire format to use.
    let type_kind = if is_borsh {
        idl::TypeKind::Borsh
    } else {
        idl::TypeKind::BytemuckRepr
    };
    let idl_type_json = if let Fields::Named(named) = fields {
        idl::build_type_json(&name_str, disc_bytes, &struct_docs, &named.named, type_kind)
    } else {
        idl::build_type_json(
            &name_str,
            disc_bytes,
            &struct_docs,
            &syn::punctuated::Punctuated::new(),
            type_kind,
        )
    };

    // Named-field types for the transitive IDL dep walk. `__register_idl_deps`
    // fans out through each field type so nested user structs land in the
    // IDL's `types[]` array even when the user never wrote `#[account]` on
    // them (e.g. a plain `#[derive(IdlType)] struct Inner` embedded in this
    // account's body).
    let idl_field_tys: Vec<&Type> = match fields {
        Fields::Named(named) => named.named.iter().map(|f| &f.ty).collect(),
        Fields::Unnamed(unnamed) => unnamed.unnamed.iter().map(|f| &f.ty).collect(),
        Fields::Unit => Vec::new(),
    };

    let (struct_attrs, pod_impls) = if is_borsh {
        (
            quote! { #[derive(anchor_lang_v2::borsh::BorshSerialize, anchor_lang_v2::borsh::BorshDeserialize, Default)] },
            quote! {},
        )
    } else {
        let field_types: Vec<_> = if let Fields::Named(named) = fields {
            named.named.iter().map(|f| &f.ty).collect()
        } else {
            vec![]
        };

        // Targeted diagnostics for common non-Pod field types. Emits a
        // `compile_error!` with a concrete suggestion instead of letting the
        // user hit an opaque `the trait bound Vec<u8>: Pod is not satisfied`.
        // Intentionally avoids recommending `#[account(borsh)]` — borsh is a
        // per-instruction serialization cost, rarely what the user actually
        // wants. The fix is almost always a Pod-compatible alternative.
        let field_diagnostics: Vec<proc_macro2::TokenStream> = if let Fields::Named(named) = fields
        {
            named
                .named
                .iter()
                .filter_map(|f| {
                    let fname = f.ident.as_ref()?.to_string();
                    let msg = diagnose_non_pod_field(&f.ty, &fname, &name_str)?;
                    let span = f.ty.span();
                    Some(quote::quote_spanned!(span=> const _: () = { compile_error!(#msg); };))
                })
                .collect()
        } else {
            Vec::new()
        };

        (
            quote! { #[derive(Clone, Copy)] #[repr(C)] },
            quote! {
                #(#field_diagnostics)*

                const _: fn() = || {
                    fn assert_pod<T: anchor_lang_v2::bytemuck::Pod>() {}
                    #( assert_pod::<#field_types>(); )*
                };
                // Verify no padding: struct size must equal sum of field sizes.
                // repr(C) inserts padding between fields with different alignments
                // (e.g. u8 followed by u64 → 7 bytes of padding). Padding bytes
                // are uninitialized, violating Pod's all-bytes-initialized requirement.
                const _: () = assert!(
                    core::mem::size_of::<#name>() == 0 #(+ core::mem::size_of::<#field_types>())*,
                    "account struct has padding bytes — reorder fields from largest to smallest alignment to eliminate padding (e.g. u64 before u32 before u8)"
                );
                unsafe impl anchor_lang_v2::bytemuck::Pod for #name {}
                unsafe impl anchor_lang_v2::bytemuck::Zeroable for #name {}
            },
        )
    };

    TokenStream::from(quote! {
        #(#attrs)*
        #struct_attrs
        #vis struct #name #fields

        #pod_impls

        impl anchor_lang_v2::Owner for #name {
            fn owner(program_id: &anchor_lang_v2::Address) -> anchor_lang_v2::Address { *program_id }
        }
        impl anchor_lang_v2::Discriminator for #name {
            const DISCRIMINATOR: &'static [u8] = &[#(#disc_literals),*];
        }
        #[cfg(feature = "idl-build")]
        impl anchor_lang_v2::IdlAccountType for #name {
            const __IDL_TYPE: Option<&'static str> = Some(#idl_type_json);
            fn __register_idl_deps(
                types: &mut ::anchor_lang_v2::__alloc::vec::Vec<&'static str>,
            ) {
                if let Some(t) = <Self as anchor_lang_v2::IdlAccountType>::__IDL_TYPE {
                    types.push(t);
                }
                #(
                    <#idl_field_tys as anchor_lang_v2::IdlAccountType>::__register_idl_deps(types);
                )*
            }
        }
    })
}

// ---------------------------------------------------------------------------
// #[derive(IdlType)]
// ---------------------------------------------------------------------------

/// Register a plain (non-`#[account]`, non-`#[event]`) struct or enum in
/// the IDL's `types[]` array.
///
/// V1 had `#[derive(IdlBuild)]` for the same purpose. Without this, a nested
/// user-defined struct or enum referenced by an `#[event]` / `#[account]`
/// field appears in the outer type's JSON as `{"defined":{"name":"Inner"}}`
/// but has no corresponding entry in `types[]` — TS clients then fail to
/// decode the nested field.
///
/// Unlike `#[account]`, this derive carries **none** of the account-kind
/// baggage: no discriminator, no `Owner`, no `Discriminator` trait, no
/// forced `#[repr(C)]`, no Pod/Zeroable derives. It only emits the
/// `IdlAccountType` impl so the type gets picked up by the transitive
/// walker.
///
/// Unions are rejected — the IDL spec has no tagged-union shape for them.
///
/// # Examples
///
/// ```ignore
/// // Plain struct dep.
/// #[repr(C)]
/// #[derive(Clone, Copy, Pod, Zeroable, IdlType)]
/// pub struct Inner { pub a: u64, pub b: u64 }
///
/// #[event]
/// pub struct NestedEvent {
///     pub outer_id: u64,
///     pub inner: Inner, // <- pulls `Inner` into the IDL's types[]
/// }
///
/// // Enum dep — unit, tuple, and struct variants all supported.
/// #[derive(IdlType)]
/// pub enum Kind {
///     Spot,
///     Futures(u64),
///     Margin { leverage: u8, symbol: [u8; 8] },
/// }
/// ```
#[proc_macro_derive(IdlType)]
pub fn derive_idl_type(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    let docs = idl::extract_doc_lines(&input.attrs);
    // `IdlType` items have no discriminator — they're just plain type
    // definitions referenced from other structs. `build_*_type_json` still
    // expect a discriminator for shape uniformity with `#[account]` /
    // `#[event]`; we pass an empty slice and strip the discriminator field
    // downstream when splitting accounts vs types entries. The spec's
    // `IdlTypeDef` doesn't carry `discriminator`, so the program-level
    // assembly already elides it when reconstructing types entries.
    let empty_disc: [u8; 0] = [];
    // Default to `Borsh` serialization (no `serialization` / `repr` fields).
    // `IdlType` is layout-agnostic — users opt into Pod separately via
    // their own `bytemuck::Pod` derive if they need zero-copy. Forcing a
    // `"bytemuck"` tag here would lie in the IDL for non-Pod types.
    let (idl_type_json, field_tys) = match &input.data {
        Data::Struct(data) => {
            let idl_json = match &data.fields {
                Fields::Named(named) => idl::build_type_json(
                    &name_str,
                    &empty_disc,
                    &docs,
                    &named.named,
                    idl::TypeKind::Borsh,
                ),
                // Unnamed (tuple) and unit structs are emitted with an empty
                // named-fields array. The spec doesn't carry a tuple-struct
                // distinction at the top level of `IdlTypeDef` — only enum
                // variants get `IdlDefinedFields::Tuple` — so tuple structs
                // fall through to struct-with-empty-fields. Users needing
                // tuple shapes should prefer named fields.
                _ => idl::build_type_json(
                    &name_str,
                    &empty_disc,
                    &docs,
                    &syn::punctuated::Punctuated::new(),
                    idl::TypeKind::Borsh,
                ),
            };
            // Walk named fields for the transitive dep registration. Unnamed
            // / unit structs contribute nothing to the walker (no inner
            // fields to recurse into) — the empty expansion is correct.
            let field_tys: Vec<&Type> = match &data.fields {
                Fields::Named(named) => named.named.iter().map(|f| &f.ty).collect(),
                Fields::Unnamed(unnamed) => unnamed.unnamed.iter().map(|f| &f.ty).collect(),
                Fields::Unit => Vec::new(),
            };
            (idl_json, field_tys)
        }
        Data::Enum(data) => {
            let idl_json = idl::build_enum_type_json(
                &name_str,
                &empty_disc,
                &docs,
                &data.variants,
                idl::TypeKind::Borsh,
            );
            // Every variant's fields contribute dependent types for the
            // transitive walker — a `Foo::Bar(Inner)` variant needs to pull
            // `Inner` into `types[]` just like a struct field would.
            let field_tys: Vec<&Type> = data
                .variants
                .iter()
                .flat_map(|v| match &v.fields {
                    Fields::Named(named) => {
                        named.named.iter().map(|f| &f.ty).collect::<Vec<_>>()
                    }
                    Fields::Unnamed(unnamed) => {
                        unnamed.unnamed.iter().map(|f| &f.ty).collect()
                    }
                    Fields::Unit => Vec::new(),
                })
                .collect();
            (idl_json, field_tys)
        }
        Data::Union(_) => {
            return syn::Error::new(
                name.span(),
                "`#[derive(IdlType)]` does not support unions — use a struct \
                 or enum instead",
            )
            .to_compile_error()
            .into();
        }
    };

    // Thread any generic / lifetime params from the input type through
    // the impl so `#[derive(IdlType)] struct Foo<'a>` lowers to
    // `impl<'a> IdlAccountType for Foo<'a>`. Without this, borrowed ix-arg
    // structs (e.g. `MixedArgs<'a> { values: &'a [u64] }`) wouldn't compile
    // with the derive.
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    TokenStream::from(quote! {
        #[cfg(feature = "idl-build")]
        impl #impl_generics anchor_lang_v2::IdlAccountType for #name #ty_generics #where_clause {
            const __IDL_TYPE: Option<&'static str> = Some(#idl_type_json);
            fn __register_idl_deps(
                types: &mut ::anchor_lang_v2::__alloc::vec::Vec<&'static str>,
            ) {
                if let Some(t) = <Self as anchor_lang_v2::IdlAccountType>::__IDL_TYPE {
                    types.push(t);
                }
                #(
                    <#field_tys as anchor_lang_v2::IdlAccountType>::__register_idl_deps(types);
                )*
            }
        }
    })
}

/// Syntactic diagnosis for common non-Pod field types on `#[account]` structs.
/// Produces a targeted, actionable error message when we can recognize the
/// shape of the offending type (Vec, String, Option, Box, bool, etc.). Falls
/// through to `None` for types we can't identify by name — the surrounding
/// `assert_pod::<T>` check in the macro output catches those generically.
///
/// Intentionally never suggests `#[account(borsh)]`: borsh accounts incur a
/// per-instruction (de)serialization cost that's rarely what a user actually
/// wants. The fix for "this field isn't Pod" is almost always a Pod-
/// compatible alternative (fixed-size array, sentinel value, `PodBool`, a
/// `Slab<H, T>` tail, etc.).
fn diagnose_non_pod_field(ty: &Type, field_name: &str, struct_name: &str) -> Option<String> {
    let Type::Path(tp) = ty else { return None };
    let seg = tp.path.segments.last()?;
    let ident = seg.ident.to_string();
    match ident.as_str() {
        "Vec" => Some(format!(
            "field `{field_name}` on `#[account] struct {struct_name}` uses `Vec`, \
             which allocates on the heap and isn't Pod. Zero-copy accounts need \
             fixed-size fields. Use `[T; N]` for a bounded array, or restructure \
             `{struct_name}` as `Slab<Header, T>` if you need a dynamic tail."
        )),
        "String" => Some(format!(
            "field `{field_name}` on `#[account] struct {struct_name}` uses \
             `String`, which allocates on the heap and isn't Pod. Use a fixed-size \
             `[u8; N]` buffer to store string data in a zero-copy account."
        )),
        "Option" => Some(format!(
            "field `{field_name}` on `#[account] struct {struct_name}` uses \
             `Option`, which carries a discriminant byte that breaks the zero-copy \
             layout contract. Use a sentinel value (e.g. an all-zero `[u8; 32]` \
             for \"no address\") or a `PodBool` flag stored alongside the value."
        )),
        "Box" | "Rc" | "Arc" => Some(format!(
            "field `{field_name}` on `#[account] struct {struct_name}` uses \
             `{ident}`, which heap-allocates and isn't valid in a zero-copy \
             account. Store the inner type directly."
        )),
        "bool" => Some(format!(
            "field `{field_name}` on `#[account] struct {struct_name}` uses \
             `bool`. `bytemuck` disallows `bool` as Pod because only `0x00` and \
             `0x01` are valid bit-patterns (any other byte read as `bool` is UB). \
             Use `anchor_lang_v2::PodBool` instead."
        )),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// #[program]
// ---------------------------------------------------------------------------

#[proc_macro_attribute]
pub fn program(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let module = parse_macro_input!(item as ItemMod);
    TokenStream::from(impl_program(&module))
}

struct HandlerCodegen {
    dispatch_arm: TokenStream2,
    wrapper: TokenStream2,
    instruction_struct: TokenStream2,
    accounts_reexport: TokenStream2,
    /// Name of the Accounts struct (e.g. `MutateItemList`). Used to dedupe
    /// `accounts::*` re-exports when multiple handlers share the same Accounts.
    accounts_type_name: String,
    idl_name: String,
    idl_disc: String,
    idl_args: String,
    /// Pre-rendered `,"docs":[...]` fragment (including the leading comma
    /// separator) that gets spliced into the per-instruction IDL JSON
    /// between `"name"` and `"discriminator"`. Empty string when the
    /// handler carries no `///` doc comments.
    idl_docs_json: String,
    idl_accounts_type: TokenStream2,
    /// Original (non-lifetime-transformed) arg types for min-length computation.
    arg_types: Vec<Type>,
}

impl HandlerCodegen {
    /// Build a codegen result that surfaces a single `compile_error!` in the
    /// emitted handler wrapper. Used when handler validation fails so the
    /// proc-macro returns a properly-spanned error instead of panicking.
    fn error(handler: &syn::ItemFn, err: syn::Error) -> Self {
        let err_tokens = err.to_compile_error();
        let fn_name = &handler.sig.ident;
        Self {
            dispatch_arm: quote! {},
            wrapper: quote! {
                #[allow(non_snake_case)]
                pub fn #fn_name() {
                    #err_tokens
                }
            },
            instruction_struct: quote! {},
            accounts_reexport: quote! {},
            accounts_type_name: String::new(),
            idl_name: fn_name.to_string(),
            idl_disc: "[]".to_string(),
            idl_args: "[]".to_string(),
            idl_docs_json: String::new(),
            idl_accounts_type: quote! { () },
            arg_types: Vec::new(),
        }
    }
}

fn process_handler(
    handler: &syn::ItemFn,
    mod_name: &Ident,
    use_byte_disc: bool,
    discrim_byte: Option<u8>,
) -> HandlerCodegen {
    let fn_name = &handler.sig.ident;
    let fn_name_str = fn_name.to_string();

    // Discriminator: 1-byte user-specified or 8-byte sha256 hash.
    use sha2::Digest;
    let hash = sha2::Sha256::digest(format!("global:{fn_name_str}").as_bytes());
    let (disc_bytes_for_idl, disc_literal_bytes, disc_match_arm_pattern): (
        Vec<u8>,
        Vec<TokenStream2>,
        TokenStream2,
    ) = if use_byte_disc {
        let byte = discrim_byte
            .expect("all-or-nothing discrim check guarantees Some when use_byte_disc is true");
        (vec![byte], vec![quote! { #byte }], quote! { #byte })
    } else {
        let disc_bytes = &hash[..8];
        let disc_u64 = u64::from_le_bytes(
            disc_bytes.try_into().expect("sha256[..8] is always 8 bytes"),
        );
        let lits: Vec<_> = disc_bytes.iter().map(|b| quote! { #b }).collect();
        (disc_bytes.to_vec(), lits, quote! { #disc_u64 })
    };
    let fn_name_log = format!("Instruction: {fn_name_str}");

    // Parse args.
    let mut args_iter = handler.sig.inputs.iter();
    let first_arg = match args_iter.next() {
        Some(arg) => arg,
        None => {
            return HandlerCodegen::error(
                handler,
                syn::Error::new(
                    handler.sig.ident.span(),
                    "handler must have a `ctx: &mut Context<T>` parameter",
                ),
            )
        }
    };
    let accounts_type = extract_context_inner_type(first_arg);

    let extra_args: Vec<(&Ident, &Type)> = args_iter
        .filter_map(|arg| {
            if let FnArg::Typed(pt) = arg {
                if let Pat::Ident(pi) = &*pt.pat {
                    return Some((&pi.ident, &*pt.ty));
                }
            }
            None
        })
        .collect();

    let extra_arg_names: Vec<_> = extra_args.iter().map(|(n, _)| *n).collect();
    let args_deser = emit_args_deser(&extra_args, "__Args", true);
    let deser_args = &args_deser.deser;
    let extra_arg_types = &args_deser.arg_types;

    // Dispatch arm.
    let dispatch_arm = quote! {
        #disc_match_arm_pattern => __handlers::#fn_name(__program_id, &mut __cursor, __ix_data, __num),
    };

    // Handler wrapper.
    let wrapper = quote! {
        #[inline(always)]
        pub fn #fn_name<'a>(
            __program_id: &'a anchor_lang_v2::Address,
            __cursor: &'a mut anchor_lang_v2::AccountCursor,
            __ix_data: &[u8],
            __num_accounts: usize,
        ) -> u64 {
            #[cfg(not(feature = "no-log-ix-name"))]
            anchor_lang_v2::msg!(#fn_name_log);
            #deser_args
            match anchor_lang_v2::run_handler::<#accounts_type>(
                __program_id,
                __cursor,
                __ix_data,
                __num_accounts,
                |__ctx| #mod_name::#fn_name(__ctx, #(#extra_arg_names),*),
            ) {
                Ok(()) => 0,
                Err(__e) => __e.into(),
            }
        }
    };

    // Client-side instruction struct.
    let ix_struct_name = syn::Ident::new(&to_camel_case(&fn_name_str), fn_name.span());
    let (ix_lt_decl, ix_lt_use) = if args_deser.has_refs {
        (quote! { <'ix> }, quote! { <'ix> })
    } else {
        (quote! {}, quote! {})
    };
    let instruction_struct = quote! {
        #[derive(anchor_lang_v2::wincode::SchemaWrite)]
        pub struct #ix_struct_name #ix_lt_decl {
            #(pub #extra_arg_names: #extra_arg_types,)*
        }
        impl #ix_lt_decl anchor_lang_v2::Discriminator for #ix_struct_name #ix_lt_use {
            const DISCRIMINATOR: &'static [u8] = &[#(#disc_literal_bytes),*];
        }
        impl #ix_lt_decl anchor_lang_v2::InstructionData for #ix_struct_name #ix_lt_use {
            fn data(&self) -> alloc::vec::Vec<u8> {
                let mut data = alloc::vec::Vec::with_capacity(256);
                data.extend_from_slice(Self::DISCRIMINATOR);
                anchor_lang_v2::wincode::serialize_into(&mut data, self)
                    .expect("instruction serialization failed");
                data
            }
        }
        impl #ix_lt_decl #ix_struct_name #ix_lt_use {
            pub fn to_instruction(
                self,
                accounts: impl anchor_lang_v2::ToAccountMetas,
            ) -> anchor_lang_v2::solana_program::instruction::Instruction {
                anchor_lang_v2::solana_program::instruction::Instruction::new_with_bytes(
                    crate::ID,
                    &<Self as anchor_lang_v2::InstructionData>::data(&self),
                    accounts.to_account_metas(None),
                )
            }
        }
    };

    // Client accounts re-export.
    let client_mod = syn::Ident::new(
        &format!(
            "__client_accounts_{}",
            accounts_type.to_string().to_lowercase()
        ),
        fn_name.span(),
    );
    let resolved_type = syn::Ident::new(
        &format!("{accounts_type}Resolved"),
        accounts_type.span(),
    );
    let accounts_reexport = quote! {
        pub use super::#client_mod::#accounts_type;
        pub use super::#client_mod::#resolved_type;
    };

    // Instruction-level docs come from `///` comments on the handler fn.
    // Leading-comma format so the IDL instruction JSON splice site can
    // hold a fixed `"{name}"{docs_or_empty},"discriminator":...` shape.
    let handler_docs = idl::extract_doc_lines(&handler.attrs);
    let idl_docs_json = if handler_docs.is_empty() {
        String::new()
    } else {
        format!(",\"docs\":{}", idl::docs_to_json_array(&handler_docs))
    };

    HandlerCodegen {
        dispatch_arm,
        wrapper,
        instruction_struct,
        accounts_reexport,
        accounts_type_name: accounts_type.to_string(),
        idl_name: fn_name_str,
        idl_disc: idl::disc_json(&disc_bytes_for_idl),
        idl_args: idl::build_args_json(&extra_args),
        idl_docs_json,
        idl_accounts_type: accounts_type,
        arg_types: extra_args.iter().map(|(_, t)| (*t).clone()).collect(),
    }
}

fn impl_program(module: &ItemMod) -> TokenStream2 {
    let mod_name = &module.ident;
    let mod_vis = &module.vis;
    let content = match &module.content {
        Some((_, items)) => items,
        None => {
            return syn::Error::new(
                module.ident.span(),
                "`#[program]` module must have an inline body",
            )
            .to_compile_error()
        }
    };

    let mut handlers = Vec::new();
    let mut other_items = Vec::new();
    for item in content {
        if let syn::Item::Fn(func) = item {
            if matches!(&func.vis, syn::Visibility::Public(_)) {
                handlers.push(func);
                continue;
            }
        }
        other_items.push(item);
    }

    // --- Parse #[discrim = N] attributes ---
    // If any handler has #[discrim = N], all must. The byte value becomes
    // the 1-byte discriminator instead of the default sha256("global:<name>")[..8].
    let discrim_attrs: Vec<Option<(u8, proc_macro2::Span)>> = match handlers
        .iter()
        .map(|h| parse_discrim_attr(h))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(v) => v,
        Err(e) => return e.to_compile_error(),
    };

    let has_any_discrim = discrim_attrs.iter().any(|d| d.is_some());
    let has_all_discrim = discrim_attrs.iter().all(|d| d.is_some());
    if has_any_discrim && !has_all_discrim {
        // Point at the first handler missing #[discrim = N] for clarity.
        let missing = handlers
            .iter()
            .zip(discrim_attrs.iter())
            .find(|(_, d)| d.is_none())
            .map(|(h, _)| h.sig.ident.span())
            .unwrap_or_else(proc_macro2::Span::call_site);
        return syn::Error::new(
            missing,
            "if any instruction in `#[program]` uses `#[discrim = N]`, all must",
        )
        .to_compile_error();
    }
    let use_byte_disc = has_any_discrim;

    if use_byte_disc {
        let mut seen: std::collections::HashMap<u8, proc_macro2::Span> =
            std::collections::HashMap::new();
        for (i, d) in discrim_attrs.iter().enumerate() {
            let (byte, span) = d.expect(
                "all-or-nothing discrim check guarantees every entry is Some",
            );
            if let Some(_first_span) = seen.insert(byte, span) {
                return syn::Error::new(
                    span,
                    format!(
                        "duplicate `#[discrim = {}]` on instruction `{}`",
                        byte, handlers[i].sig.ident
                    ),
                )
                .to_compile_error();
            }
        }
    }
    let discrim_attrs: Vec<Option<u8>> = discrim_attrs.iter().map(|d| d.map(|(b, _)| b)).collect();

    let codegen: Vec<HandlerCodegen> = handlers
        .iter()
        .enumerate()
        .map(|(i, h)| process_handler(h, mod_name, use_byte_disc, discrim_attrs[i]))
        .collect();

    let dispatch_arms: Vec<_> = codegen.iter().map(|c| &c.dispatch_arm).collect();
    let handler_wrappers: Vec<_> = codegen.iter().map(|c| &c.wrapper).collect();
    let instruction_structs: Vec<_> = codegen.iter().map(|c| &c.instruction_struct).collect();
    // Dedupe `accounts` re-exports: multiple handlers sharing the same
    // Accounts struct would otherwise emit duplicate `pub use` statements.
    let accounts_reexports: Vec<_> = {
        let mut seen = std::collections::HashSet::new();
        codegen
            .iter()
            .filter(|c| seen.insert(c.accounts_type_name.clone()))
            .map(|c| &c.accounts_reexport)
            .collect()
    };
    let idl_ix_names: Vec<_> = codegen.iter().map(|c| &c.idl_name).collect();
    let idl_ix_discs: Vec<_> = codegen.iter().map(|c| &c.idl_disc).collect();
    let idl_ix_args: Vec<_> = codegen.iter().map(|c| &c.idl_args).collect();
    let idl_ix_docs: Vec<_> = codegen.iter().map(|c| &c.idl_docs_json).collect();
    let idl_accounts_types: Vec<_> = codegen.iter().map(|c| &c.idl_accounts_type).collect();
    // One `Vec<Type>` per handler, for the ix-arg transitive IDL walker.
    // Nested `#(...)*` inside the quote! sees this as a Vec<Vec<Type>> and
    // expands the outer level over handlers, the inner level over each
    // handler's arg types. `arg_types` includes every handler parameter
    // past `ctx: &mut Context<...>`, so primitives / &T / Vec / etc. all
    // flow through (with the blanket no-op impls for non-`IdlType` types).
    let ix_arg_types_per_handler: Vec<&Vec<Type>> =
        codegen.iter().map(|c| &c.arg_types).collect();
    let all_ix_arg_types: Vec<_> = codegen.iter().map(|c| &c.arg_types).collect();

    // Generate disc parsing code based on mode.
    // Returns u64 error code on failure (not Err) since __anchor_dispatch
    // returns u64 directly.
    // Build a const expression for the minimum ix_data length across all
    // instructions: disc_size + min(serialized args size per ix). Uses
    // `size_of` on a tuple of arg types — only when ALL args are owned
    // fixed-size types (no references, no dynamic-size). Falls back to 0
    // for instructions with references or complex types.
    fn is_fixed_size_primitive(ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(p) if p.path.segments.len() == 1 => {
                let name = p.path.segments[0].ident.to_string();
                matches!(
                    name.as_str(),
                    "u8" | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "i8"
                        | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "bool"
                )
            }
            _ => false,
        }
    }
    let min_args_size_expr = if all_ix_arg_types.is_empty() {
        quote! { 0usize }
    } else {
        let per_ix_sizes: Vec<_> = all_ix_arg_types
            .iter()
            .map(|types| {
                if types.is_empty() || !types.iter().all(is_fixed_size_primitive) {
                    quote! { 0usize }
                } else {
                    quote! { core::mem::size_of::<(#(#types,)*)>() }
                }
            })
            .collect();
        quote! { {
            const __SIZES: &[usize] = &[#(#per_ix_sizes),*];
            const fn __const_min(s: &[usize]) -> usize {
                let mut m = s[0];
                let mut i = 1;
                while i < s.len() { if s[i] < m { m = s[i]; } i += 1; }
                m
            }
            __const_min(__SIZES)
        } }
    };
    let disc_size: usize = if use_byte_disc { 1 } else { 8 };

    let disc_parse = if use_byte_disc {
        quote! {
            const __MIN_IX_DATA_LEN: usize = #disc_size + #min_args_size_expr;
            if __ix_data_len < __MIN_IX_DATA_LEN {
                return anchor_lang_v2::Error::from(
                    anchor_lang_v2::ErrorCode::InstructionFallbackNotFound,
                ).into();
            }
            let __disc: u8 = *__ix_data_ptr;
            let __ix_data: &[u8] =
                ::core::slice::from_raw_parts(__ix_data_ptr.add(1), __ix_data_len - 1);
        }
    } else {
        quote! {
            if __ix_data_len < 8 {
                return anchor_lang_v2::Error::from(
                    anchor_lang_v2::ErrorCode::InstructionFallbackNotFound,
                ).into();
            }
            let __disc: u64 = u64::from_le_bytes(
                *(__ix_data_ptr as *const [u8; 8])
            );
            let __ix_data: &[u8] =
                ::core::slice::from_raw_parts(__ix_data_ptr.add(8), __ix_data_len - 8);
        }
    };

    // Strip #[discrim = N] attributes from handler outputs so rustc
    // doesn't complain about an unknown attribute.
    let handlers: Vec<_> = handlers
        .iter()
        .map(|func| {
            let mut func = (*func).clone();
            func.attrs.retain(|attr| {
                if let syn::Meta::NameValue(nv) = &attr.meta {
                    !nv.path.is_ident("discrim")
                } else {
                    true
                }
            });
            func
        })
        .collect();

    quote! {
        #mod_vis mod #mod_name {
            #(#other_items)*
            #(#handlers)*
        }

        // Custom 2-arg (r1, r2) entrypoint using SIMD-0321 convention.
        #[cfg(not(feature = "no-entrypoint"))]
        anchor_lang_v2::pinocchio::default_allocator!();
        #[cfg(not(feature = "no-entrypoint"))]
        anchor_lang_v2::pinocchio::default_panic_handler!();

        /// Matches Solana's transaction-wide account cap (u8 index space).
        /// The lookup array holds `[AccountView; 256]` = ~2 KiB of frame
        /// used for duplicate-account resolution during cursor walks.
        const __ANCHOR_MAX_ACCOUNTS: usize = 256;

        /// Core dispatch: program-id check, discriminator parse, account
        /// cursor setup, handler dispatch. Exported as the `entrypoint`
        /// symbol unless `no-entrypoint` is active. Custom entrypoints
        /// can call this directly via its Rust path.
        ///
        /// The BPF loader passes:
        ///   r1 = MM_INPUT_START (first byte of the serialized parameter region)
        ///   r2 = VM address of the instruction data bytes (SIMD-0321)
        ///
        /// The `[r2 - 8 .. r2]` slot holds the `u64` ix_data length and the
        /// 32 bytes at `[r2 + len .. +32]` hold the program_id, per agave's
        /// aligned serialization layout (see `solana-program-runtime
        /// ::serialization::serialize_parameters_aligned`).
        #[cfg_attr(not(feature = "no-entrypoint"), export_name = "entrypoint")]
        pub unsafe extern "C" fn __anchor_dispatch(
            __input: *mut u8,
            __ix_data_ptr: *const u8,
        ) -> u64 {
            let __ix_data_len = *(__ix_data_ptr.sub(8) as *const u64) as usize;
            let __program_id: &anchor_lang_v2::Address =
                &*(__ix_data_ptr.add(__ix_data_len) as *const anchor_lang_v2::Address);

            if let Err(__e) = anchor_lang_v2::check_program_id(__program_id, &crate::ID) {
                return __e.into();
            }

            // Parse the discriminator.
            #disc_parse

            let __num = *(__input as *const u64) as usize;
            if let Err(__e) = anchor_lang_v2::check_max_accounts(__num, __ANCHOR_MAX_ACCOUNTS) {
                return __e.into();
            }

            let mut __lookup: [::core::mem::MaybeUninit<anchor_lang_v2::AccountView>;
                __ANCHOR_MAX_ACCOUNTS] =
                [const { ::core::mem::MaybeUninit::uninit() }; __ANCHOR_MAX_ACCOUNTS];
            let mut __cursor = anchor_lang_v2::AccountCursor::new(
                __input,
                __lookup.as_mut_ptr() as *mut anchor_lang_v2::AccountView,
            );

            // Each dispatch arm returns u64 directly (0 = success).
            match __disc {
                #(#dispatch_arms)*
                _ => anchor_lang_v2::Error::from(
                    anchor_lang_v2::ErrorCode::InstructionFallbackNotFound,
                ).into(),
            }
        }

        mod __handlers {
            use super::*;
            use anchor_lang_v2::TryAccounts as _;
            #(#handler_wrappers)*
        }

        /// Client-side instruction structs for off-chain use.
        pub mod instruction {
            extern crate alloc;
            use super::*;
            use anchor_lang_v2::Discriminator as _;
            #(#instruction_structs)*
        }

        /// Client-side accounts structs (re-exports) for off-chain use.
        pub mod accounts {
            #(#accounts_reexports)*
        }

        // IDL generation: prints structured output consumed by `anchor idl build`.
        // The CLI runs `cargo test __anchor_private_print_idl --features idl-build`
        // and parses the marker-delimited sections from stdout.
        #[cfg(all(test, feature = "idl-build"))]
        mod __anchor_private_idl {
            use super::*;

            #[test]
            fn __anchor_private_print_idl_address() {
                println!("--- IDL begin address ---");
                let addr = crate::ID;
                // Print base58 address
                println!("{}", anchor_lang_v2::Address::from(addr));
                println!("--- IDL end address ---");
            }

            #[test]
            fn __anchor_private_print_idl_program() {
                let instructions = vec![
                    #(
                        format!(
                            "{{\"name\":\"{}\"{},\"discriminator\":{},\"accounts\":{},\"args\":{}}}",
                            #idl_ix_names,
                            #idl_ix_docs,
                            #idl_ix_discs,
                            #idl_accounts_types::__idl_accounts(),
                            #idl_ix_args,
                        )
                    ),*
                ];

                // Collect types from all accounts structs via the transitive
                // dep walker. `__register_idl_deps` on each field type pushes
                // the field's `__IDL_TYPE` (if any) and recursively walks its
                // own fields — so a plain `#[derive(IdlType)] struct Inner`
                // embedded in an `#[account]` data type lands in `types[]`
                // too. View wrappers (Signer, Program<T>, Sysvar<T>, …) use
                // the trait's default no-op `__register_idl_deps`, so they
                // contribute nothing.
                //
                // Also walk every handler's **ix arg types** — a user
                // struct referenced only as a `#[program]` fn argument
                // (e.g. `args: MixedArgs<'_>`) otherwise lands in
                // `instructions[].args` as a bare `{defined:{name:...}}`
                // reference with no matching `types[]` entry. Primitive /
                // collection / `&T` blanket impls are no-op forwarders
                // (`idl_build.rs`), so only user-derived `IdlType` structs
                // contribute.
                let mut all_types: Vec<&str> = Vec::new();
                #(#idl_accounts_types::__idl_register_deps(&mut all_types);)*
                #(
                    #(
                        <#ix_arg_types_per_handler as anchor_lang_v2::IdlAccountType>::__register_idl_deps(&mut all_types);
                    )*
                )*
                all_types.sort();
                all_types.dedup();

                // Split each __IDL_TYPE into an `IdlAccount`
                // (name + discriminator, spec:137-140) and an
                // `IdlTypeDef` (everything else, spec:176-188). Each
                // input string is a full type def:
                //
                //   {"name":"X","discriminator":[...][,"docs":[...]]
                //    [,"serialization":"...","repr":{...}],
                //    "type":{"kind":"struct","fields":[...]}}
                //
                // We parse each one as structured JSON, pluck off
                // `name` + `discriminator` for the accounts entry, drop
                // `discriminator` for the types entry, and skip
                // `IdlType`-registered plain types (empty discriminator)
                // from `accounts[]` entirely.
                let mut accounts_entries = Vec::new();
                let mut types_entries = Vec::new();
                for ty in &all_types {
                    let Ok(mut parsed) =
                        anchor_lang_v2::__serde_json::from_str::<
                            anchor_lang_v2::__serde_json::Value,
                        >(ty)
                    else {
                        continue;
                    };
                    let Some(obj) = parsed.as_object_mut() else { continue; };
                    let Some(name) = obj.get("name").cloned() else { continue; };
                    let disc = obj.remove("discriminator");
                    let disc_is_empty = matches!(
                        &disc,
                        Some(anchor_lang_v2::__serde_json::Value::Array(a)) if a.is_empty()
                    );
                    if let Some(disc_v) = disc {
                        if !disc_is_empty {
                            let acct = anchor_lang_v2::__serde_json::json!({
                                "name": name,
                                "discriminator": disc_v,
                            });
                            accounts_entries.push(acct.to_string());
                        }
                    }
                    // `parsed` is now `{name, docs?, serialization?, repr?, type}` —
                    // exactly the `IdlTypeDef` shape (spec:176-188).
                    types_entries.push(parsed.to_string());
                }

                let crate_name = env!("CARGO_CRATE_NAME").replace('-', "_");
                // Pull `description` / `repository` from the program crate's
                // Cargo.toml via `option_env!`. Cargo sets `CARGO_PKG_*` when
                // it invokes the compiler; unset or empty strings get dropped
                // from the JSON (the spec marks both fields
                // `skip_serializing_if` in `IdlMetadata`). `dependencies` stays
                // unimplemented — it needs a workspace traversal v1 handles
                // via a separate tool.
                let description = option_env!("CARGO_PKG_DESCRIPTION")
                    .filter(|s| !s.is_empty());
                let repository = option_env!("CARGO_PKG_REPOSITORY")
                    .filter(|s| !s.is_empty());
                let mut metadata_extras = anchor_lang_v2::__alloc::string::String::new();
                if let Some(d) = description {
                    // Escape embedded quotes/backslashes so the JSON stays valid.
                    let escaped = d.replace('\\', "\\\\").replace('"', "\\\"");
                    metadata_extras.push_str(&anchor_lang_v2::__alloc::format!(
                        ",\"description\":\"{}\"",
                        escaped,
                    ));
                }
                if let Some(r) = repository {
                    let escaped = r.replace('\\', "\\\\").replace('"', "\\\"");
                    metadata_extras.push_str(&anchor_lang_v2::__alloc::format!(
                        ",\"repository\":\"{}\"",
                        escaped,
                    ));
                }
                let idl = format!(
                    "{{\"address\":\"\",\"metadata\":{{\"name\":\"{}\",\"version\":\"0.1.0\",\"spec\":\"0.1.0\"{}}},\"instructions\":[{}],\"accounts\":[{}],\"types\":[{}]}}",
                    crate_name,
                    metadata_extras,
                    instructions.join(","),
                    accounts_entries.join(","),
                    types_entries.join(","),
                );
                println!("--- IDL begin program ---");
                println!("{}", idl);
                println!("--- IDL end program ---");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// #[event]
// ---------------------------------------------------------------------------

/// Attribute macro that marks a struct as an event.
///
/// Three modes:
///
/// **Default (`#[event]`, wincode).** Derives `wincode::SchemaWrite` and
/// serializes via `wincode::serialize_into`. Supports arbitrary layouts,
/// including `Vec`/`String`/`Option`/enums, and is materially cheaper than
/// borsh on SBF (see `cu-bench` — roughly 3–10× fewer CUs depending on
/// shape). This is the right default for almost every event.
///
/// **`#[event(bytemuck)]`.** Emits `#[repr(C)]` + a raw `copy_nonoverlapping`
/// of the struct bytes. Fastest of the three for fixed-size events, but the
/// struct must contain only fixed-size, non-fat-pointer fields (no
/// `Vec`/`String`/`Box`/`Option`/enums/maps) and must have zero `repr(C)`
/// padding. Both invariants are enforced at compile time. Opt into this only
/// when the event is on a hot path and profiling shows wincode's per-field
/// encoding is the bottleneck.
///
/// **`#[event(borsh)]`.** Emits a `borsh::BorshSerialize` derive. Matches v1
/// semantics and wire format. Retained for IDL compatibility with existing
/// off-chain consumers that decode via borsh; prefer the default otherwise.
///
/// All three modes share the same discriminator and `Event::data()` contract,
/// so `emit!` works identically. The IDL carries a `serialization` tag so TS
/// clients know how to decode.
///
/// # Examples
///
/// ```ignore
/// // Default: wincode, supports dynamic fields.
/// #[event]
/// pub struct DepositRecorded {
///     pub ledger: [u8; 32],
///     pub amount: u64,
///     pub memo: String,
/// }
///
/// // Opt-in bytemuck: cheapest on fixed-size hot-path events.
/// #[event(bytemuck)]
/// pub struct TickUpdate {
///     pub price: u64,
///     pub ts: u64,
/// }
///
/// // Opt-in borsh: v1-compatible wire format.
/// #[event(borsh)]
/// pub struct MetadataUpdated {
///     pub uri: String,
///     pub tags: Vec<[u8; 32]>,
/// }
/// ```
#[proc_macro_attribute]
pub fn event(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mode = match parse_event_mode(attr) {
        Ok(mode) => mode,
        Err(err) => return err.to_compile_error().into(),
    };

    let input = parse_macro_input!(item as DeriveInput);
    let name = input.ident.clone();
    let name_str = name.to_string();
    let vis = &input.vis;
    let attrs = &input.attrs;
    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => {
            return syn::Error::new(name.span(), "`#[event]` only supports structs")
                .to_compile_error()
                .into()
        }
    };

    use sha2::Digest;
    let hash = sha2::Sha256::digest(format!("event:{name_str}").as_bytes());
    let disc_bytes = &hash[..8];
    let disc_literals: Vec<_> = disc_bytes.iter().map(|b| quote! { #b }).collect();

    let discriminator_impl = quote! {
        impl anchor_lang_v2::Discriminator for #name {
            const DISCRIMINATOR: &'static [u8] = &[#(#disc_literals),*];
        }
    };

    // Build the `--- IDL begin event ---` payload. `idl/src/build.rs` expects
    // a JSON object with `event: IdlEvent` (name + discriminator) plus
    // `types: Vec<IdlTypeDef>` (the full struct definition). Each mode tags
    // its own serialization; borsh uses the spec default (skip_serializing_if
    // strips it), bytemuck adds `{serialization:"bytemuck",repr:{kind:"c"}}`,
    // wincode uses the `Custom(String)` escape hatch.
    let type_kind = match mode {
        EventMode::Wincode => idl::TypeKind::Wincode,
        EventMode::Bytemuck => idl::TypeKind::BytemuckRepr,
        EventMode::Borsh => idl::TypeKind::Borsh,
    };
    let struct_docs = idl::extract_doc_lines(attrs);
    let type_def_json = if let Fields::Named(named) = fields {
        idl::build_type_json(&name_str, disc_bytes, &struct_docs, &named.named, type_kind)
    } else {
        idl::build_type_json(
            &name_str,
            disc_bytes,
            &struct_docs,
            &syn::punctuated::Punctuated::new(),
            type_kind,
        )
    };
    let event_disc_json = idl::disc_json(disc_bytes);
    let event_header_json = format!(
        "{{\"event\":{{\"name\":\"{}\",\"discriminator\":{}}},\"types\":[",
        name_str, event_disc_json,
    );
    // Field types for the transitive type walk. The event itself pushes
    // `type_def_json` into the types accumulator via its `__IDL_TYPE`
    // const; field-type deps fan out from there so plain user structs
    // referenced by `pub inner: Inner` land in `types[]` too.
    let idl_field_tys: Vec<&Type> = match fields {
        Fields::Named(named) => named.named.iter().map(|f| &f.ty).collect(),
        Fields::Unnamed(unnamed) => unnamed.unnamed.iter().map(|f| &f.ty).collect(),
        Fields::Unit => Vec::new(),
    };
    let idl_fn_name = quote::format_ident!(
        "__anchor_private_print_idl_event_{}",
        name_str.to_lowercase()
    );
    // The event's own `IdlAccountType` impl (emitted further down) owns
    // `__IDL_TYPE = Some(#type_def_json)` and a `__register_idl_deps` that
    // pushes both self and every field-type dep. The test here just walks
    // that deps list at runtime and assembles the `--- IDL begin event ---`
    // payload — so `types[]` picks up nested user structs without the test
    // having to know about them.
    let idl_event_print = quote! {
        #[cfg(all(test, feature = "idl-build"))]
        #[test]
        fn #idl_fn_name() {
            let mut __types: anchor_lang_v2::__alloc::vec::Vec<&'static str> =
                anchor_lang_v2::__alloc::vec::Vec::new();
            <#name as anchor_lang_v2::IdlAccountType>::__register_idl_deps(&mut __types);
            __types.sort();
            __types.dedup();
            let mut __payload = anchor_lang_v2::__alloc::string::String::from(#event_header_json);
            let mut __first = true;
            for __t in &__types {
                if !__first { __payload.push(','); }
                __first = false;
                __payload.push_str(__t);
            }
            __payload.push_str("]}");
            println!("--- IDL begin event ---");
            println!("{}", __payload);
            println!("--- IDL end event ---");
        }
    };
    // Emit the `IdlAccountType` impl for the event itself. Walks field
    // types for transitive dep registration. Same shape as the `#[account]`
    // impl — the only difference is that `#[event]` + `#[account]` both
    // carry their own discriminator embedded in `type_def_json`.
    let idl_account_type_impl = quote! {
        #[cfg(feature = "idl-build")]
        impl anchor_lang_v2::IdlAccountType for #name {
            const __IDL_TYPE: Option<&'static str> = Some(#type_def_json);
            fn __register_idl_deps(
                types: &mut ::anchor_lang_v2::__alloc::vec::Vec<&'static str>,
            ) {
                if let Some(t) = <Self as anchor_lang_v2::IdlAccountType>::__IDL_TYPE {
                    types.push(t);
                }
                #(
                    <#idl_field_tys as anchor_lang_v2::IdlAccountType>::__register_idl_deps(types);
                )*
            }
        }
    };

    match mode {
        EventMode::Wincode => TokenStream::from(quote! {
            // `#[derive(wincode::SchemaWrite)]` lays down the per-field encoder.
            // No `repr(C)` — wincode is layout-agnostic (it walks the derived
            // schema, not the in-memory byte layout) so the compiler is free
            // to pick whichever Rust layout is best.
            #[derive(anchor_lang_v2::wincode::SchemaWrite)]
            #(#attrs)*
            #vis struct #name #fields

            #discriminator_impl

            impl anchor_lang_v2::Event for #name {
                fn data(&self) -> anchor_lang_v2::__alloc::vec::Vec<u8> {
                    let disc = <Self as anchor_lang_v2::Discriminator>::DISCRIMINATOR;
                    // 256-byte preallocation matches the instruction-data
                    // emission site (derive/src/lib.rs ~line 971). Wincode
                    // has no `encoded_size()` yet, so this is a best-guess
                    // that avoids a reallocation for typical event shapes.
                    let mut buf = anchor_lang_v2::__alloc::vec::Vec::with_capacity(
                        disc.len() + 256,
                    );
                    buf.extend_from_slice(disc);
                    anchor_lang_v2::wincode::serialize_into(&mut buf, self)
                        .expect("`#[event]` wincode serialization cannot fail for \
                                 derived SchemaWrite types");
                    buf
                }
            }

            #idl_account_type_impl

            #idl_event_print
        }),
        EventMode::Bytemuck => {
            let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();

            // Targeted diagnostics for common non-Pod field types. Fires
            // *before* the generic `assert_pod::<T>` bound so users hit a
            // field-specific migration hint instead of the opaque
            // `Vec<u8>: Pod is not satisfied` error. Mirrors the pattern in
            // `#[account]` zero-copy codegen. Borsh mode is suggested here
            // because (unlike `#[account]`) events have a correct dynamic
            // fallback — see `diagnose_non_pod_event_field`.
            let field_diagnostics: Vec<_> = fields
                .iter()
                .filter_map(|field| {
                    let field_name =
                        field.ident.as_ref().map(|i| i.to_string()).unwrap_or_default();
                    let msg = diagnose_non_pod_event_field(&field.ty, &field_name)?;
                    Some(quote! { ::core::compile_error!(#msg); })
                })
                .collect();

            // Transitive Pod bound per field. Catches any fat-pointer or
            // uninit-byte-containing type, including ones hidden inside user-
            // defined structs — `bytemuck::Pod` is recursively checked at the
            // bound site, so `struct User { v: Vec<u8> }` fails here even
            // though the derive macro can't see through `User`.
            //
            // Padding check: `repr(C)` inserts alignment padding between
            // fields of differing alignment. Padding bytes are uninitialized,
            // which violates `Pod`'s all-bytes-initialized requirement and
            // would also silently drift from a borsh-decoded client view.
            // The assertion tells the author how to fix it.
            //
            // Finally, `unsafe impl Pod + Zeroable for Self` so consumers can
            // `bytemuck::from_bytes` the logged payload directly — mirrors
            // `#[account]`'s zero-copy output shape.
            TokenStream::from(quote! {
                #[repr(C)]
                #[derive(::core::clone::Clone, ::core::marker::Copy)]
                #(#attrs)*
                #vis struct #name #fields

                // Targeted diagnostics fire first so users see a specific
                // migration hint (e.g. "use `#[event(borsh)]` for dynamic
                // strings") instead of bytemuck's opaque `Pod not satisfied`.
                #(#field_diagnostics)*

                // Transitive Pod bound per field — catches fat pointers even
                // when hidden inside an opaque user-defined struct (the
                // `Pod` trait propagates through nested types).
                const _: fn() = || {
                    fn assert_pod<T: anchor_lang_v2::bytemuck::Pod>() {}
                    #( assert_pod::<#field_types>(); )*
                };

                // `repr(C)` padding is target-dependent: on SBF `u128` is
                // align-8, so a `{Address (align 1), u64, u128}` struct has
                // no padding; on x86_64 hosts `u128` is align-16, inserting
                // a phantom 8-byte gap before the `u128`. Gating on the
                // Solana target means `cargo check` accepts the struct
                // based on BPF layout (the only layout that actually
                // ships) and `cargo build-sbf` still enforces the no-
                // padding invariant.
                #[cfg(target_os = "solana")]
                const _: () = ::core::assert!(
                    ::core::mem::size_of::<#name>()
                        == 0 #( + ::core::mem::size_of::<#field_types>() )*,
                    "`#[event]` struct has `repr(C)` alignment padding — \
                     reorder fields from largest to smallest alignment (u128/u64 \
                     first, then u32, then u16, then u8/bool), or switch to \
                     `#[event(borsh)]` for arbitrary layouts"
                );

                // SAFETY: `bytemuck::Pod` requires four invariants. Each is
                // proven by a compile-time check earlier in this block:
                //
                //   (1) `#[repr(C)]`                     — enforced by the
                //       `#[repr(C)]` attribute emitted above.
                //   (2) Every field is `Pod`             — enforced by the
                //       `assert_pod::<T>()` ghost fn. Failure is
                //       `T: Pod is not satisfied`, which transitively
                //       rejects fat pointers (`Vec`, `String`, `Box`, `&T`),
                //       uninit-byte types (`bool`, enums, `Option`), and
                //       any user struct that itself isn't `Pod`.
                //   (3) No interior padding bytes        — enforced by the
                //       `size_of::<Self>() == Σ size_of::<Field>()` assert
                //       under `cfg(target_os = "solana")`. Padding bytes
                //       are `MaybeUninit`, which would be read by
                //       `bytemuck::bytes_of` / `bytemuck::from_bytes` and
                //       constitute UB — the assert precludes the case.
                //   (4) `Copy` + `'static`               — `Copy` is
                //       derived above; `'static` is required by
                //       `assert_pod::<T: 'static>` transitively.
                //
                // The cfg-gated padding assert deliberately only evaluates
                // on the Solana target. `repr(C)` padding is target-
                // dependent: on SBF `u128` is align-8, so `{Address (align
                // 1), u64, u128}` is perfectly packed; on x86_64 hosts
                // `u128` is align-16, so that same layout has a phantom
                // 8-byte gap during `cargo check`. This is not a soundness
                // hole — the event bytes only get memcpy'd into
                // `sol_log_data` on the actual deployment target, where
                // the assert does run.
                //
                // Not using `#[derive(Pod)]` because bytemuck's own
                // padding check is unconditional (not target-gated) and
                // would reject `u128`-carrying events on host compile even
                // though their on-chain layout is sound.
                unsafe impl anchor_lang_v2::bytemuck::Pod for #name {}
                unsafe impl anchor_lang_v2::bytemuck::Zeroable for #name {}

                #discriminator_impl

                impl anchor_lang_v2::Event for #name {
                    fn data(&self) -> anchor_lang_v2::__alloc::vec::Vec<u8> {
                        const SIZE: usize = ::core::mem::size_of::<#name>();
                        let disc = <Self as anchor_lang_v2::Discriminator>::DISCRIMINATOR;
                        let mut buf = anchor_lang_v2::__alloc::vec::Vec::with_capacity(
                            disc.len() + SIZE,
                        );
                        buf.extend_from_slice(disc);
                        let start = buf.len();
                        buf.resize(start + SIZE, 0);
                        unsafe {
                            ::core::ptr::copy_nonoverlapping(
                                self as *const Self as *const u8,
                                buf.as_mut_ptr().add(start),
                                SIZE,
                            );
                        }
                        buf
                    }
                }

                #idl_account_type_impl

                #idl_event_print
            })
        }
        EventMode::Borsh => TokenStream::from(quote! {
            // No `repr(C)` — borsh is layout-agnostic, and letting the compiler
            // pick layout leaves room for future niche optimizations.
            //
            // `#[borsh(crate = "…")]` points borsh's derive at the re-export
            // path so user crates don't need `borsh` as a direct dependency —
            // `anchor-lang-v2` already pins the version.
            #[derive(anchor_lang_v2::borsh::BorshSerialize)]
            #[borsh(crate = "anchor_lang_v2::borsh")]
            #(#attrs)*
            #vis struct #name #fields

            #discriminator_impl

            impl anchor_lang_v2::Event for #name {
                fn data(&self) -> anchor_lang_v2::__alloc::vec::Vec<u8> {
                    let disc = <Self as anchor_lang_v2::Discriminator>::DISCRIMINATOR;
                    let mut buf = anchor_lang_v2::__alloc::vec::Vec::with_capacity(disc.len() + 64);
                    buf.extend_from_slice(disc);
                    <Self as anchor_lang_v2::borsh::BorshSerialize>::serialize(self, &mut buf)
                        .expect("`#[event(borsh)]` serialization cannot fail for \
                                 standard borsh types");
                    buf
                }
            }

            #idl_account_type_impl

            #idl_event_print
        }),
    }
}

enum EventMode {
    Wincode,
    Bytemuck,
    Borsh,
}

/// Parse the `#[account]` attribute's optional mode argument.
///
/// Accepts: `#[account]` (default → zero-copy Pod) or `#[account(borsh)]`.
/// Previously this was `attr.to_string().contains("borsh")`, which silently
/// accepted typos like `#[account(borhs)]` as zero-copy — the struct would
/// compile under the wrong layout and clients decoding it as borsh would
/// get garbage.
fn parse_account_mode(attr: TokenStream) -> Result<bool, syn::Error> {
    if attr.is_empty() {
        return Ok(false);
    }
    let attr2: proc_macro2::TokenStream = attr.into();
    let ident: syn::Ident = syn::parse2(attr2.clone()).map_err(|_| {
        syn::Error::new_spanned(
            &attr2,
            "expected `#[account]` or `#[account(borsh)]` — no other \
             arguments are supported",
        )
    })?;
    if ident == "borsh" {
        Ok(true)
    } else {
        Err(syn::Error::new_spanned(
            ident,
            "unknown `#[account]` mode — only `borsh` is accepted",
        ))
    }
}

fn parse_event_mode(attr: TokenStream) -> Result<EventMode, syn::Error> {
    if attr.is_empty() {
        return Ok(EventMode::Wincode);
    }
    let attr2: proc_macro2::TokenStream = attr.into();
    let ident: syn::Ident = syn::parse2(attr2.clone()).map_err(|_| {
        syn::Error::new_spanned(
            &attr2,
            "expected `#[event]`, `#[event(bytemuck)]`, or `#[event(borsh)]` — \
             no other arguments are supported",
        )
    })?;
    if ident == "bytemuck" {
        Ok(EventMode::Bytemuck)
    } else if ident == "borsh" {
        Ok(EventMode::Borsh)
    } else {
        Err(syn::Error::new_spanned(
            ident,
            "unknown `#[event]` mode — only `bytemuck` and `borsh` are accepted",
        ))
    }
}

/// Targeted diagnostics for common non-Pod field types on
/// `#[event(bytemuck)]` structs. Bytemuck mode is strict by design —
/// `Vec`/`String`/`Option`/etc. can't round-trip through a `copy_nonoverlapping`
/// of the struct bytes. The hint steers authors to drop the `bytemuck` flag
/// (getting the wincode default, which handles these fine). Returns `None`
/// for types we can't recognize by name — the `assert_pod::<T>` bound
/// catches those generically via `bytemuck::Pod`.
fn diagnose_non_pod_event_field(ty: &Type, field_name: &str) -> Option<String> {
    let Type::Path(tp) = ty else { return None };
    let seg = tp.path.segments.last()?;
    let ident = seg.ident.to_string();
    match ident.as_str() {
        "Vec" => Some(format!(
            "event field `{field_name}` uses `Vec`, which is a fat pointer — the \
             `#[event(bytemuck)]` memcpy path would emit the `(ptr, len, cap)` \
             bits instead of the elements. Use `[T; N]` for a fixed-size array, \
             or drop the `bytemuck` attribute to use the default wincode \
             encoding, which handles `Vec` natively."
        )),
        "String" => Some(format!(
            "event field `{field_name}` uses `String`, which is a fat pointer — \
             the `#[event(bytemuck)]` memcpy path would emit the `(ptr, len, \
             cap)` bits instead of the UTF-8 bytes. Use `[u8; N]` for a bounded \
             buffer, or drop the `bytemuck` attribute to use the default \
             wincode encoding, which handles `String` natively."
        )),
        "Option" => Some(format!(
            "event field `{field_name}` uses `Option`, whose niche-or-tag \
             layout isn't guaranteed to match the client decoder. Use a \
             sentinel value (e.g. an all-zero `[u8; 32]` for \"no address\"), \
             or drop the `bytemuck` attribute to use the default wincode \
             encoding."
        )),
        "Box" | "Rc" | "Arc" | "Cow" | "Weak" => Some(format!(
            "event field `{field_name}` uses `{ident}`, which is a heap/shared \
             pointer — its bytes are a pointer, not the referenced data. \
             Inline the value directly (`T` instead of `{ident}<T>`), or drop \
             the `bytemuck` attribute to use the default wincode encoding."
        )),
        "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet" | "BinaryHeap"
        | "LinkedList" | "VecDeque" => Some(format!(
            "event field `{field_name}` uses `{ident}`, which allocates on the \
             heap. Drop the `bytemuck` attribute to use the default wincode \
             encoding, which handles dynamic collections."
        )),
        "bool" => Some(format!(
            "event field `{field_name}` is `bool`. `bytemuck` disallows `bool` \
             as Pod because only `0x00` and `0x01` are valid — any other byte \
             is UB. Use a `u8` and treat `0` / non-zero as the boolean, or \
             drop the `bytemuck` attribute to use the default wincode encoding."
        )),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// emit!
// ---------------------------------------------------------------------------

/// Logs an event that can be subscribed to by clients.
///
/// Uses the `sol_log_data` syscall which emits a `Program data: <Base64>` log.
///
/// # Example
///
/// ```ignore
/// emit!(DepositRecorded { ledger: *ctx.accounts.ledger.account().address(), amount });
/// ```
#[proc_macro]
pub fn emit(input: TokenStream) -> TokenStream {
    let data: proc_macro2::TokenStream = input.into();
    TokenStream::from(quote! {
        {
            anchor_lang_v2::sol_log_data(&[&anchor_lang_v2::Event::data(&#data)]);
        }
    })
}

// ---------------------------------------------------------------------------
// #[access_control]
// ---------------------------------------------------------------------------

/// Executes the given access control method before running the decorated
/// instruction handler. Any method in scope of the attribute can be invoked
/// with any arguments from the associated instruction handler.
///
/// # Example
///
/// ```ignore
/// #[program]
/// mod errors {
///     use super::*;
///
///     #[access_control(Create::validate(&ctx, bump_seed))]
///     pub fn create(ctx: &mut Context<Create>, bump_seed: u8) -> Result<()> {
///         ctx.accounts.my_account.bump_seed = bump_seed;
///         Ok(())
///     }
/// }
///
/// impl Create {
///     pub fn validate(ctx: &Context<Create>, bump_seed: u8) -> Result<()> {
///         // ... custom validation ...
///         Ok(())
///     }
/// }
/// ```
///
/// This pattern is useful for invariants that depend on instruction
/// arguments — `#[derive(Accounts)]` constraints fire before args are
/// unpacked, so any check that needs both an account and an arg goes
/// here.
#[proc_macro_attribute]
pub fn access_control(args: TokenStream, input: TokenStream) -> TokenStream {
    access_control::expand(args, input)
}

// ---------------------------------------------------------------------------
// #[constant]
// ---------------------------------------------------------------------------

/// Marker attribute for `pub const` items that should appear in the generated
/// IDL. Does nothing at runtime. When the `idl-build` feature is enabled, a
/// companion test function emits the constant's metadata for `anchor idl build`.
///
/// # Example
///
/// ```ignore
/// #[constant]
/// pub const SEED: &str = "anchor";
/// ```
#[proc_macro_attribute]
pub fn constant(_attr: TokenStream, input: TokenStream) -> TokenStream {
    constant::expand(input)
}

// ---------------------------------------------------------------------------
// #[derive(InitSpace)]
// ---------------------------------------------------------------------------

/// Implements [`anchor_lang_v2::Space`] on the decorated struct or enum so
/// users can write `space = 8 + MyAccount::INIT_SPACE` in `#[account(init)]`.
///
/// Variable-size fields (`String`, `Vec<T>`) require a `#[max_len(N)]` helper
/// attribute to specify the reserved capacity.
///
/// # Example
///
/// ```ignore
/// #[account(borsh)]
/// #[derive(InitSpace)]
/// pub struct Profile {
///     pub owner: Address,
///     #[max_len(32)]
///     pub name: String,
/// }
/// ```
#[proc_macro_derive(InitSpace, attributes(max_len))]
pub fn derive_init_space(item: TokenStream) -> TokenStream {
    init_space::expand(item)
}

// ---------------------------------------------------------------------------
// #[error_code]
// ---------------------------------------------------------------------------

/// Port of v1's `#[error_code]` without the runtime `AnchorError` heap
/// allocations. Emits `impl From<E> for Error` returning
/// `Error::Custom(variant_index + offset)`. `#[msg("text")]` is IDL-only.
///
/// # Example
///
/// ```ignore
/// #[error_code]
/// pub enum MyError {
///     #[msg("invalid threshold")]
///     InvalidThreshold,
///     TooManySigners,
/// }
///
/// // usage:
/// return Err(MyError::InvalidThreshold.into());
/// ```
///
/// Supports `#[error_code(offset = N)]` for the first code (default 6000).
#[proc_macro_attribute]
pub fn error_code(args: TokenStream, input: TokenStream) -> TokenStream {
    error_code::expand(args, input)
}

/// Parse the optional `#[discrim = N]` attribute on a handler fn.
/// Returns `Ok(Some((byte, span)))` if present, `Ok(None)` if absent,
/// or `Err` with a properly-spanned diagnostic on malformed input.
fn parse_discrim_attr(
    handler: &syn::ItemFn,
) -> syn::Result<Option<(u8, proc_macro2::Span)>> {
    for attr in &handler.attrs {
        if let syn::Meta::NameValue(nv) = &attr.meta {
            if nv.path.is_ident("discrim") {
                let span = nv.value.span();
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(lit),
                    ..
                }) = &nv.value
                {
                    let byte = lit.base10_parse::<u8>().map_err(|_| {
                        syn::Error::new(lit.span(), "`#[discrim = N]` value must fit in a u8 (0..=255)")
                    })?;
                    return Ok(Some((byte, span)));
                }
                return Err(syn::Error::new(
                    span,
                    "`#[discrim = N]` value must be an integer literal",
                ));
            }
        }
    }
    Ok(None)
}

fn extract_context_inner_type(arg: &FnArg) -> TokenStream2 {
    let ty = match arg {
        FnArg::Typed(pt) => &*pt.ty,
        _ => {
            return syn::Error::new(
                arg.span(),
                "first parameter must be `ctx: &mut Context<T>`",
            )
            .to_compile_error()
        }
    };
    if let Type::Reference(r) = ty {
        return extract_generic_arg(&r.elem);
    }
    extract_generic_arg(ty)
}

fn extract_generic_arg(ty: &Type) -> TokenStream2 {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                for arg in &args.args {
                    if let syn::GenericArgument::Type(inner) = arg {
                        return quote! { #inner };
                    }
                }
            }
        }
    }
    syn::Error::new(
        ty.span(),
        "could not extract generic type from `Context<T>` - expected `Context<YourAccountsStruct>`",
    )
    .to_compile_error()
}

/// Converts `snake_case` to `CamelCase` (e.g. `execute_transfer` → `ExecuteTransfer`).
fn to_camel_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}
