use {
    proc_macro2::TokenStream as TokenStream2,
    quote::quote,
    syn::{
        ext::IdentExt,
        parse::ParseStream,
        Attribute, Expr, Ident, Token, Type,
    },
};

/// A namespaced constraint like `token::mint = expr`.
pub struct NamespacedConstraint {
    /// e.g. "token"
    pub namespace: String,
    /// e.g. "MintConstraint" (capitalized + suffixed for Constrain trait lookup)
    pub key: String,
    /// e.g. "mint" (original lowercase key, used as init param field name)
    pub raw_key: String,
    /// The RHS expression.
    pub value: Expr,
    /// True if the RHS is a simple ident (field reference → call .account()).
    /// False if it's a literal or complex expression (pass directly).
    pub is_field_ref: bool,
}

pub struct AccountAttrs {
    pub is_mut: bool,
    pub is_signer: bool,
    pub is_init: bool,
    pub is_init_if_needed: bool,
    pub is_zeroed: bool,
    pub is_executable: bool,
    pub is_dup: bool,
    /// None = not specified, Some(true) = enforce, Some(false) = skip
    pub rent_exempt: Option<bool>,
    /// None = no bump attr, Some(None) = `bump` without value, Some(Some(expr)) = `bump = expr`
    pub bump: Option<Option<Expr>>,
    pub payer: Option<Ident>,
    pub space: Option<Expr>,
    pub seeds: Option<Expr>,
    /// Override program_id for PDA derivation: `seeds::program = expr`
    pub seeds_program: Option<Expr>,
    pub has_one: Vec<(Ident, Option<Expr>)>,
    pub address: Option<Expr>,
    pub address_error: Option<Expr>,
    pub owner: Option<Expr>,
    pub owner_error: Option<Expr>,
    pub close: Option<Ident>,
    pub constraint: Option<Expr>,
    pub constraint_error: Option<Expr>,
    pub realloc: Option<Expr>,
    pub realloc_payer: Option<Ident>,
    pub realloc_zero: bool,
    /// Namespaced constraints: token::mint, mint::authority, etc.
    pub namespaced: Vec<NamespacedConstraint>,
}

/// PDA metadata produced by seed classification. Each entry is a pre-built
/// JSON string matching the `IdlSeed` enum shape (`{"kind":"const"...}`
/// etc.). `program` mirrors the optional `seeds::program = expr` override.
#[derive(Clone, Debug)]
pub struct IdlPdaMeta {
    pub seeds: Vec<String>,
    pub program: Option<String>,
}

pub fn parse_account_attrs(attrs: &[Attribute]) -> syn::Result<AccountAttrs> {
    let mut result = AccountAttrs {
        is_mut: false,
        is_signer: false,
        is_init: false,
        is_init_if_needed: false,
        is_zeroed: false,
        is_executable: false,
        is_dup: false,
        rent_exempt: None,
        bump: None,
        payer: None,
        space: None,
        seeds: None,
        seeds_program: None,
        has_one: Vec::new(),
        address: None,
        address_error: None,
        owner: None,
        owner_error: None,
        close: None,
        constraint: None,
        constraint_error: None,
        realloc: None,
        realloc_payer: None,
        realloc_zero: false,
        namespaced: Vec::new(),
    };

    for attr in attrs {
        if !attr.path().is_ident("account") {
            continue;
        }
        attr.parse_args_with(|input: ParseStream| {
            while !input.is_empty() {
                let ident = Ident::parse_any(input)?;
                match ident.to_string().as_str() {
                    "mut" => result.is_mut = true,
                    "init" => {
                        result.is_init = true;
                        result.is_mut = true;
                    }
                    "init_if_needed" => {
                        result.is_init_if_needed = true;
                        result.is_mut = true;
                    }
                    "zeroed" => {
                        result.is_zeroed = true;
                        result.is_mut = true;
                    }
                    "bump" => {
                        if input.peek(Token![=]) {
                            input.parse::<Token![=]>()?;
                            result.bump = Some(Some(input.parse()?));
                        } else {
                            result.bump = Some(None);
                        }
                    }
                    "signer" => result.is_signer = true,
                    "executable" => result.is_executable = true,
                    "dup" => {
                        return Err(syn::Error::new(
                            ident.span(),
                            "`dup` bypasses duplicate-account safety checks and must be \
                             explicitly marked unsafe: use `unsafe(dup)`",
                        ));
                    }
                    "unsafe" => {
                        let content;
                        syn::parenthesized!(content in input);
                        let inner: Ident = content.parse()?;
                        match inner.to_string().as_str() {
                            "dup" => {
                                result.is_dup = true;
                                result.is_mut = true;
                            }
                            _ => {
                                return Err(syn::Error::new(
                                    inner.span(),
                                    format!("unknown unsafe constraint `{inner}`"),
                                ));
                            }
                        }
                    }
                    "rent_exempt" => {
                        input.parse::<Token![=]>()?;
                        let val: Ident = input.parse()?;
                        result.rent_exempt = Some(val == "enforce");
                    }
                    "payer" => {
                        input.parse::<Token![=]>()?;
                        result.payer = Some(input.parse()?);
                    }
                    "space" => {
                        input.parse::<Token![=]>()?;
                        result.space = Some(input.parse()?);
                    }
                    "seeds" if input.peek(Token![=]) => {
                        input.parse::<Token![=]>()?;
                        result.seeds = Some(input.parse()?);
                    }
                    // `seeds::program = expr` falls through to the
                    // namespaced-path handler below. Adding an explicit
                    // `seeds` arm without a peek check would eat the `seeds`
                    // ident and then fail to parse the following `::`.
                    "has_one" => {
                        input.parse::<Token![=]>()?;
                        let target: Ident = input.parse()?;
                        let err = if input.peek(Token![@]) {
                            input.parse::<Token![@]>()?;
                            Some(input.parse()?)
                        } else {
                            None
                        };
                        result.has_one.push((target, err));
                    }
                    "address" => {
                        input.parse::<Token![=]>()?;
                        result.address = Some(input.parse()?);
                        if input.peek(Token![@]) {
                            input.parse::<Token![@]>()?;
                            result.address_error = Some(input.parse()?);
                        }
                    }
                    "owner" => {
                        input.parse::<Token![=]>()?;
                        result.owner = Some(input.parse()?);
                        if input.peek(Token![@]) {
                            input.parse::<Token![@]>()?;
                            result.owner_error = Some(input.parse()?);
                        }
                    }
                    "realloc" => {
                        input.parse::<Token![=]>()?;
                        result.realloc = Some(input.parse()?);
                        result.is_mut = true;
                    }
                    "realloc_payer" => {
                        input.parse::<Token![=]>()?;
                        result.realloc_payer = Some(input.parse()?);
                    }
                    "realloc_zero" => {
                        input.parse::<Token![=]>()?;
                        let val: syn::LitBool = input.parse()?;
                        result.realloc_zero = val.value;
                    }
                    "close" => {
                        input.parse::<Token![=]>()?;
                        result.close = Some(input.parse()?);
                    }
                    "constraint" => {
                        input.parse::<Token![=]>()?;
                        result.constraint = Some(input.parse()?);
                        // Optional: @ ErrorExpr
                        if input.peek(Token![@]) {
                            input.parse::<Token![@]>()?;
                            result.constraint_error = Some(input.parse()?);
                        }
                    }
                    _ => {
                        // Check for namespaced constraint: namespace::key = value
                        if input.peek(Token![::]) {
                            input.parse::<Token![::]>()?;
                            let key_ident = Ident::parse_any(input)?;
                            // seeds::program = expr — special case, stored separately
                            if ident == "seeds" && key_ident == "program" {
                                input.parse::<Token![=]>()?;
                                result.seeds_program = Some(input.parse()?);
                                if !input.is_empty() {
                                    input.parse::<Token![,]>()?;
                                }
                                continue;
                            }
                            input.parse::<Token![=]>()?;
                            // Peek to determine if RHS is a simple ident (field ref)
                            // or a literal/expression (value).
                            let is_field_ref = input.peek(syn::Ident);
                            let value: Expr = input.parse()?;
                            // snake_case → PascalCase + Constraint suffix:
                            //   "mint"             → "MintConstraint"
                            //   "freeze_authority" → "FreezeAuthorityConstraint"
                            //   "min_stake"        → "MinStakeConstraint"
                            // Previous behaviour only capitalised the first
                            // char, which produced `Freeze_authorityConstraint`
                            // — unusable as a Rust type name. External crates
                            // following the spl-v2 pattern assume idiomatic
                            // PascalCase, so snake-segment joining is the
                            // correct resolution.
                            let key = {
                                let s = key_ident.to_string();
                                let mut out = String::with_capacity(s.len() + "Constraint".len());
                                let mut upper_next = true;
                                for ch in s.chars() {
                                    if ch == '_' {
                                        upper_next = true;
                                    } else if upper_next {
                                        out.extend(ch.to_uppercase());
                                        upper_next = false;
                                    } else {
                                        out.push(ch);
                                    }
                                }
                                out.push_str("Constraint");
                                out
                            };
                            let raw_key = key_ident.to_string();
                            result.namespaced.push(NamespacedConstraint {
                                namespace: ident.to_string(),
                                key,
                                raw_key,
                                value,
                                is_field_ref,
                            });
                        } else {
                            // No `::` follows — not a namespaced constraint.
                            // Reject to catch typos like `singler` instead of `signer`.
                            return Err(syn::Error::new(
                                ident.span(),
                                format!("unknown account constraint `{ident}`"),
                            ));
                        }
                    }
                }
                if !input.is_empty() {
                    input.parse::<Token![,]>()?;
                }
            }
            Ok(())
        })?;
    }
    Ok(result)
}

pub fn field_ty_str(ty: &Type) -> String {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident.to_string();
        }
    }
    String::new()
}

/// Namespaced constraints whose impls are runtime-only and should NOT be
/// threaded as init-time `Params` fields. External crates add their own
/// runtime-only namespaces here (e.g. `dynamic_account` from
/// `anchor-dynamic`).
pub fn is_runtime_only_constraint_ns(ns: &str) -> bool {
    matches!(ns, "dynamic_account")
}

pub fn is_nested_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Nested";
        }
    }
    false
}

/// Pull the first generic arg out of a `Nested<T>` type path, e.g.
/// `Nested<InnerAccounts>` → `InnerAccounts`. Returns `None` for anything
/// else. Used by the `HEADER_SIZE` codegen to walk into nested account
/// structs and sum their compile-time header counts.
pub fn extract_nested_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Nested" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    None
}

/// Extracts the inner `T` from `Option<T>` for optional-account field detection.
/// Users write `pub foo: Option<Account<Bar>>` in their Accounts struct; the
/// derive constructs `None` when the client passes the program's own address
/// as the sentinel, otherwise `Some(Bar::load(view)?)`.
pub fn extract_option_inner(ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    None
}

pub struct AccountField {
    pub name: Ident,
    /// The field's original `syn::Type` — used by `impl_accounts` to build
    /// the `HEADER_SIZE` compile-time sum (1 per direct field, +
    /// `<Inner as TryAccounts>::HEADER_SIZE` per `Nested<Inner>`).
    pub ty: Type,
    pub load: TokenStream2,
    pub constraints: Vec<TokenStream2>,
    /// Duplicate-mutable-account check. Collected separately from
    /// `constraints` so all mut-field dup checks can share a single outer
    /// `if let Some(__dups) = __duplicates` gate — non-dup txs pay one
    /// Option-tag branch regardless of field count.
    pub dup_check: Option<TokenStream2>,
    pub exit: Option<TokenStream2>,
    pub has_bump: bool,
    /// True when the field type is `Option<T>` (optional account).
    pub is_optional: bool,
    // IDL metadata
    pub idl_writable: bool,
    /// True when this is a fresh-keypair init site (attrs: `init` or
    /// `init_if_needed` without `seeds`). The caller must sign the tx with
    /// the new account's keypair, so it surfaces as `signer: true` in the
    /// IDL. Orthogonal to the `Signer` field type — those contribute via
    /// `<Ty as IdlAccountType>::__IDL_IS_SIGNER` at runtime.
    pub idl_init_signer: bool,
    /// `has_one = target` targets declared on this field's attrs. Relations
    /// emission walks every field and looks for has_one chains targeting
    /// *another* field, so we need to keep them addressable per-source to
    /// build the inverse mapping (matches v1's `get_relations`).
    pub idl_has_one: Vec<String>,
    /// Extracted `#[doc = "..."]` lines on the field, in source order.
    /// Emitted as `"docs":[...]` in the accounts JSON. Matches
    /// `IdlInstructionAccount.docs` (`idl/spec/src/lib.rs:83`).
    pub idl_docs: Vec<String>,
    /// Classified seed metadata for PDA emission. `None` when the field
    /// has no `seeds = [...]` attr.
    pub idl_pda: Option<IdlPdaMeta>,
    /// The raw field type, post-`Option<T>` unwrap. Used by the generated
    /// `__idl_types()` function to dispatch `<Ty as IdlAccountType>::__IDL_TYPE`
    /// on the wrapper type (`Program<T>`, `Account<T>`, …) rather than on its
    /// `::Data` associated type. `None` only for non-`Type::Path` fields that
    /// can't appear as accounts (defensive — this path shouldn't trigger in
    /// practice).
    pub idl_field_ty: Option<syn::Type>,
}

/// Rewrite a single seed expression so that a bare field-name identifier
/// (like `wallet` in `seeds = [b"vault", wallet]`) is replaced with the
/// explicit byte-slice derivation chain `wallet.address().as_ref()`.
///
/// Strict: only rewrites simple single-segment `Expr::Path` expressions
/// whose identifier matches a known field name. Everything else
/// (literals, method calls, array refs, complex expressions) passes
/// through unchanged so users can still write explicit seed expressions.
fn rewrite_seed_expr(expr: &Expr, field_names: &[String]) -> proc_macro2::TokenStream {
    use quote::quote;
    if let Expr::Path(ep) = expr {
        if ep.qself.is_none() && ep.path.segments.len() == 1 && ep.path.leading_colon.is_none() {
            let seg = &ep.path.segments[0];
            if seg.arguments.is_empty() {
                let ident = &seg.ident;
                if field_names.contains(&ident.to_string()) {
                    return quote! { #ident.address().as_ref() };
                }
            }
        }
    }
    quote! { #expr }
}

/// Build the seed-check codegen for a `#[account(seeds = [..], bump)]`
/// field. Tries to precompute the canonical PDA bump at macro-expansion
/// time when all seeds are byte literals and the crate's program id can
/// be discovered from `src/lib.rs`, emitting `verify_program_address`
/// in place of the runtime `find_program_address` loop.
///
/// Falls back to the dynamic path whenever:
///   - any seed is non-literal (field reference, method call, expr),
///   - `seeds::program = expr` overrides the derivation program id, or
///   - program-id discovery fails for any reason (missing lib.rs,
///     parse error, no `declare_id!` macro, malformed argument).
///
/// `target_addr_ref` must be a TokenStream producing `&Address` for the
/// account whose address we're verifying: `__target.address()` inside
/// the `init` arm, `<field>.account().address()` for non-init
/// constraints.
///
/// `for_init = true` additionally emits the `let __seeds: Option<&[&[u8]]> = Some(...)`
/// binding in the enclosing scope, as required by the init arm's
/// subsequent `create_and_initialize` call.
///
/// `using_our_program_id = false` (i.e. `seeds::program = ...` is set)
/// unconditionally falls back to the dynamic path, since we only know
/// how to discover our own crate's `declare_id!`.
#[allow(clippy::too_many_arguments)]
fn emit_seeds_check(
    seeds: &[&Expr],
    seed_exprs: &[TokenStream2],
    pda_program: &TokenStream2,
    target_addr_ref: &TokenStream2,
    field_name: &Ident,
    field_ty: Option<&Type>,
    for_init: bool,
    using_our_program_id: bool,
    is_optional: bool,
) -> TokenStream2 {
    // For optional fields the bumps struct field is `Option<u8>`, so the
    // assignment wraps in `Some(...)`. Non-optional fields assign the bump
    // directly.
    let wrap_bump = |b: TokenStream2| -> TokenStream2 {
        if is_optional {
            quote! { Some(#b) }
        } else {
            b
        }
    };
    // Try to precompute the bump and PDA at expansion time.
    if using_our_program_id {
        if let Some(literal_seeds) = crate::pda::seeds_as_byte_literals(seeds) {
            if let Some(program_id) = crate::pda::discover_program_id() {
                let seed_slices: Vec<&[u8]> = literal_seeds.iter().map(|s| s.as_slice()).collect();
                if let Some((bump, pda_bytes)) =
                    crate::pda::precompute_pda(&seed_slices, &program_id)
                {
                    // Field-scoped const names keep multiple fields'
                    // bumps + PDAs from colliding, even when two
                    // constraints share an outer scope.
                    let upper = field_name.to_string().to_uppercase();
                    let bump_const = Ident::new(&format!("__{}_BUMP", upper), field_name.span());
                    let pda_const = Ident::new(&format!("__{}_PDA", upper), field_name.span());
                    // Emit the 32-byte PDA as an `Address` const.
                    let pda_bytes_tokens = pda_bytes.iter().map(|b| quote! { #b });
                    let bump_assign = wrap_bump(quote! { #bump_const });
                    let check = quote! {
                        const #bump_const: u8 = #bump;
                        const #pda_const: anchor_lang_v2::Address =
                            anchor_lang_v2::Address::new_from_array([#(#pda_bytes_tokens),*]);
                        if !anchor_lang_v2::address_eq(#target_addr_ref, &#pda_const) {
                            return Err(anchor_lang_v2::ErrorCode::ConstraintSeeds.into());
                        }
                        __bumps.#field_name = #bump_assign;
                    };
                    return if for_init {
                        quote! {
                            #check
                            let __seeds: Option<&[&[u8]]> =
                                Some(&[#(#seed_exprs),* , &[#bump_const]]);
                        }
                    } else {
                        // Wrap non-init in a block so the consts are
                        // scoped and can't collide with other fields.
                        quote! { { #check } }
                    };
                }
            }
        }
    }

    // Fallback: runtime find loop fused with the equality check.
    //
    // Skip `sol_curve_validate_point` when the account is provably
    // signed-for (init path or MIN_DATA_LEN > 0), since CreateAccount
    // already validates the PDA via `create_program_address`.
    //
    // Otherwise (`UncheckedAccount` with zero data, non-init): the curve
    // check is the only proof the address is a real PDA.
    //
    // `MIN_DATA_LEN` is a trait const, so the branch is resolved at
    // compile time — LLVM eliminates the dead path entirely.
    // TODO: decide whether init paths should assume the subsequent
    // CreateAccount CPI guarantees the address is off-curve, letting
    // us skip `sol_curve_validate_point`. Currently we always run the
    // curve check on init to avoid relying on the trait impl's CPI.
    let skip_curve = if let Some(ty) = field_ty {
        quote! { <#ty as anchor_lang_v2::AnchorAccount>::MIN_DATA_LEN > 0 }
    } else {
        quote! { false }
    };
    let bump_assign = wrap_bump(quote! { __bump });
    let find = quote! {
        let __bump = if #skip_curve {
            anchor_lang_v2::find_and_verify_program_address_skip_curve(
                &[#(#seed_exprs),*], #pda_program, #target_addr_ref,
            ).map_err(|_| anchor_lang_v2::ErrorCode::ConstraintSeeds)?
        } else {
            anchor_lang_v2::find_and_verify_program_address(
                &[#(#seed_exprs),*], #pda_program, #target_addr_ref,
            ).map_err(|_| anchor_lang_v2::ErrorCode::ConstraintSeeds)?
        };
        __bumps.#field_name = #bump_assign;
    };
    if for_init {
        quote! {
            #find
            let __seeds: Option<&[&[u8]]> = Some(&[#(#seed_exprs),* , &[__bump]]);
        }
    } else {
        find
    }
}

/// Emit the shared init body used by both `#[account(init)]` and
/// `#[account(init_if_needed)]`: seeds check, param assignments,
/// `create_and_initialize`, and `load_mut_after_init`.
fn emit_init_body(
    field_name: &Ident,
    field_ty: &Type,
    attrs: &AccountAttrs,
    field_names: &[String],
    is_optional: bool,
) -> TokenStream2 {
    let payer = attrs.payer.as_ref().expect("init requires payer");
    // Fall back to `<T as Space>::INIT_SPACE` when `space` is omitted.
    // SPL types (Mint, TokenAccount) impl Space = size_of<Self>() so
    // `#[account(init, token::mint = ..., token::authority = ...)]` works
    // without hardcoding magic numbers like `space = 165`.
    let space = match attrs.space.as_ref() {
        Some(expr) => quote! { #expr },
        None => quote! { <#field_ty as anchor_lang_v2::Space>::INIT_SPACE },
    };

    // Init params come from namespaced constraints that name init-time
    // inputs (e.g. `mint::authority = x`). Runtime-only constraints —
    // currently any constraint whose Params type has no matching field —
    // would fail to typecheck if threaded here. We filter out the ones
    // we know are runtime-only before collecting param assignments.
    let param_assignments: Vec<_> = attrs
        .namespaced
        .iter()
        .filter(|nc| !is_runtime_only_constraint_ns(&nc.namespace))
        .map(|nc| {
            let key = Ident::new(&nc.raw_key, proc_macro2::Span::call_site());
            let value = &nc.value;
            if nc.is_field_ref {
                quote! { __p.#key = Some(#value.account()); }
            } else {
                quote! { __p.#key = Some(#value); }
            }
        })
        .collect();

    let seeds_arg = if let Some(ref seeds_expr) = attrs.seeds {
        let using_our_program_id = attrs.seeds_program.is_none();
        let pda_program = match &attrs.seeds_program {
            Some(prog) => quote! { &#prog },
            None => quote! { __program_id },
        };
        if let Expr::Array(arr) = seeds_expr {
            let seed_elems: Vec<&Expr> = arr.elems.iter().collect();
            let rewritten: Vec<_> = seed_elems
                .iter()
                .map(|s| rewrite_seed_expr(s, field_names))
                .collect();
            emit_seeds_check(
                &seed_elems,
                &rewritten,
                &pda_program,
                &quote! { __target.address() },
                field_name,
                None,
                true,
                using_our_program_id,
                is_optional,
            )
        } else {
            // Opaque expression seeds — runtime find + verify.
            let bump_assign = if is_optional {
                quote! { Some(__bump) }
            } else {
                quote! { __bump }
            };
            quote! {
                let __seed_expr_val = #seeds_expr;
                let __seed_ref: &[&[u8]] = __seed_expr_val.as_ref();
                let __bump =
                    anchor_lang_v2::find_and_verify_program_address(
                        __seed_ref, #pda_program, &__target.address(),
                    ).map_err(|_| anchor_lang_v2::ErrorCode::ConstraintSeeds)?;
                __bumps.#field_name = #bump_assign;
                let mut __seed_buf: [&[u8]; 17] = [&[]; 17];
                let __n = __seed_ref.len();
                __seed_buf[..__n].copy_from_slice(__seed_ref);
                __seed_buf[__n] = &[__bump];
                let __seeds: Option<&[&[u8]]> = Some(&__seed_buf[..__n + 1]);
            }
        }
    } else {
        quote! { let __seeds: Option<&[&[u8]]> = None; }
    };

    quote! {
        let __payer = #payer.account();
        #seeds_arg
        let __init_params = {
            type __P<'__a> = <#field_ty as anchor_lang_v2::AccountInitialize>::Params<'__a>;
            let mut __p = <__P as Default>::default();
            #(#param_assignments)*
            __p
        };
        <#field_ty as anchor_lang_v2::AccountInitialize>::create_and_initialize(
            __payer, &__target, #space, __program_id, &__init_params, __seeds,
        )?
    }
}

pub fn parse_field(
    field: &syn::Field,
    field_names: &[String],
    offset_expr: proc_macro2::TokenStream,
    ix_arg_names: &[String],
) -> syn::Result<AccountField> {
    let field_name = field.ident.as_ref().expect("named field");
    let field_ty = &field.ty;
    let attrs = parse_account_attrs(&field.attrs)?;

    let option_inner = extract_option_inner(field_ty);
    let is_optional = option_inner.is_some();
    // Fresh-keypair init (no seeds) — caller signs the tx. Distinct from
    // `Signer`-type fields, which the IDL picks up through
    // `IdlAccountType::__IDL_IS_SIGNER` at runtime.
    let idl_init_signer = (attrs.is_init || attrs.is_init_if_needed) && attrs.seeds.is_none();
    let idl_writable = attrs.is_mut;
    let idl_has_one: Vec<String> = attrs.has_one.iter().map(|(i, _)| i.to_string()).collect();
    let idl_docs = crate::idl::extract_doc_lines(&field.attrs);
    let idl_pda = attrs.seeds.as_ref().map(|seeds_expr| {
        let seed_entries = if let Expr::Array(arr) = seeds_expr {
            arr.elems
                .iter()
                .map(|s| crate::idl::classify_seed(s, field_names, ix_arg_names))
                .collect()
        } else {
            vec![r#"{"kind":"expr"}"#.to_string()]
        };
        IdlPdaMeta {
            seeds: seed_entries,
            program: attrs
                .seeds_program
                .as_ref()
                .map(|p| crate::idl::classify_seed(p, field_names, ix_arg_names)),
        }
    });
    let idl_field_ty: Option<syn::Type> = {
        let base_ty = option_inner.unwrap_or(field_ty);
        if let Type::Path(_) = base_ty {
            Some(base_ty.clone())
        } else {
            None
        }
    };

    let has_bump = attrs.seeds.is_some();

    // --- Load ---
    if is_nested_type(field_ty) {
        let inner_ty = extract_nested_inner_type(field_ty)
            .expect("is_nested_type was true but extract_nested_inner_type returned None");
        // Nested<Inner> — delegate to Inner::try_accounts, which advances the
        // shared cursor by Inner::HEADER_SIZE. The outer walk_n covers only
        // direct (non-nested) fields; the nested try_accounts picks up where
        // the outer left off.
        //
        // Constraint processing and exit are handled by the inner struct's own
        // try_accounts / exit_accounts — the outer derives don't need to
        // re-check them.
        // TODO: passing `__base_offset + #offset_expr` means the nested
        // struct's bitvec lookups hit the correct global indices. This is
        // correct but adds a runtime addition per dup-check inside the
        // nested struct. A future optimization could pre-shift the bitvec
        // or use a wrapper that offsets transparently.
        let load = quote! {
            let (__nested_inner, _) =
                <#inner_ty as anchor_lang_v2::TryAccounts>::try_accounts(
                    __program_id,
                    &__views[#offset_expr .. #offset_expr + <#inner_ty as anchor_lang_v2::TryAccounts>::HEADER_SIZE],
                    __duplicates,
                    __base_offset + #offset_expr,
                    __ix_data,
                )?;
            let #field_name = anchor_lang_v2::Nested(__nested_inner);
        };
        let exit = Some(quote! {
            self.#field_name.0.exit_accounts()?;
        });
        return Ok(AccountField {
            name: field_name.clone(),
            ty: field.ty.clone(),
            load,
            constraints: vec![],
            dup_check: None,
            exit,
            has_bump: false,
            is_optional: false,
            idl_writable: false,
            idl_init_signer: false,
            idl_has_one: vec![],
            idl_docs: vec![],
            idl_pda: None,
            idl_field_ty: None,
        });
    }

    let load = if let Some(inner_ty) = option_inner {
        // `Option<T>` field: client-side sentinel of "account address ==
        // program_id" is interpreted as `None`. Otherwise we run the same
        // load / init / init_if_needed / zeroed logic we would for a
        // non-optional `T`, but against `inner_ty` (so the v2 trait-based
        // `AccountInitialize` / `AnchorAccount` impls dispatch on `T`, not
        // `Option<T>`), and wrap the result in `Some`.
        let inner_action = if attrs.is_init {
            // Init body emitted against inner_ty so the trait call lands on T.
            let init_body = emit_init_body(field_name, inner_ty, &attrs, field_names, true);
            quote! { Some({ #init_body }) }
        } else if attrs.is_init_if_needed {
            let init_body = emit_init_body(field_name, inner_ty, &attrs, field_names, true);
            quote! {
                if __target.data_len() > 0
                    && !__target.owned_by(&anchor_lang_v2::programs::System::id())
                {
                    // SAFETY: the bitvec duplicate-account check below ensures
                    // no other mutable reference to this account's data exists.
                    Some(unsafe {
                        <#inner_ty as anchor_lang_v2::AnchorAccount>::load_mut(
                            __target, __program_id,
                        )?
                    })
                } else {
                    Some({ #init_body })
                }
            }
        } else if attrs.is_zeroed {
            quote! {
                {
                    let __disc = <#inner_ty as anchor_lang_v2::Discriminator>::DISCRIMINATOR;
                    {
                        let __data = __target.try_borrow()?;
                        if __data.len() < __disc.len()
                            || __data[..__disc.len()].iter().any(|b| *b != 0)
                        {
                            return Err(anchor_lang_v2::ErrorCode::ConstraintZero.into());
                        }
                    }
                    unsafe {
                        let mut __view = __target;
                        let __data = __view.borrow_unchecked_mut();
                        __data[..__disc.len()].copy_from_slice(__disc);
                    }
                    // SAFETY: the bitvec duplicate-account check below ensures
                    // no other mutable reference to this account's data exists.
                    Some(unsafe {
                        <#inner_ty as anchor_lang_v2::AnchorAccount>::load_mut(
                            __target, __program_id,
                        )?
                    })
                }
            }
        } else if attrs.is_mut {
            quote! {
                // SAFETY: the bitvec duplicate-account check below ensures
                // no other mutable reference to this account's data exists.
                Some(unsafe {
                    <#inner_ty as anchor_lang_v2::AnchorAccount>::load_mut(
                        __target, __program_id,
                    )?
                })
            }
        } else {
            quote! {
                Some(<#inner_ty as anchor_lang_v2::AnchorAccount>::load(
                    __target, __program_id,
                )?)
            }
        };
        quote! {
            let mut #field_name: #field_ty = {
                let __target = __views[#offset_expr];
                if anchor_lang_v2::address_eq(__target.address(), __program_id) {
                    None
                } else {
                    #inner_action
                }
            };
        }
    } else if attrs.is_init {
        let init_body = emit_init_body(field_name, field_ty, &attrs, field_names, false);
        quote! {
            let mut #field_name: #field_ty = {
                let __target = __views[#offset_expr];
                #init_body
            };
        }
    } else if attrs.is_init_if_needed {
        let init_body = emit_init_body(field_name, field_ty, &attrs, field_names, false);
        quote! {
            let mut #field_name: #field_ty = {
                let __target = __views[#offset_expr];
                if __target.data_len() > 0 && !__target.owned_by(&anchor_lang_v2::programs::System::id()) {
                    // SAFETY: the bitvec duplicate-account check below ensures
                    // no other mutable reference to this account's data exists.
                    unsafe { <#field_ty as anchor_lang_v2::AnchorAccount>::load_mut(__target, __program_id)? }
                } else {
                    #init_body
                }
            };
        }
    } else if attrs.is_zeroed {
        // zeroed: account exists but discriminator must be all zeros. Verify,
        // stamp the real discriminator, then load mutably.
        quote! {
            let mut #field_name: #field_ty = {
                let __target = __views[#offset_expr];
                let __disc = <#field_ty as anchor_lang_v2::Discriminator>::DISCRIMINATOR;
                {
                    let __data = __target.try_borrow()?;
                    if __data.len() < __disc.len() || __data[..__disc.len()].iter().any(|b| *b != 0) {
                        return Err(anchor_lang_v2::ErrorCode::ConstraintZero.into());
                    }
                }
                unsafe {
                    let mut __view = __target;
                    let __data = __view.borrow_unchecked_mut();
                    __data[..__disc.len()].copy_from_slice(__disc);
                }
                // SAFETY: the bitvec duplicate-account check below ensures
                // no other mutable reference to this account's data exists.
                unsafe { <#field_ty as anchor_lang_v2::AnchorAccount>::load_mut(__target, __program_id)? }
            };
        }
    } else if attrs.is_mut {
        quote! {
            // SAFETY: the bitvec duplicate-account check below ensures no
            // other mutable reference to this account's data exists.
            let mut #field_name = unsafe { <#field_ty as anchor_lang_v2::AnchorAccount>::load_mut(__views[#offset_expr], __program_id)? };
        }
    } else {
        quote! {
            let #field_name: #field_ty = anchor_lang_v2::AnchorAccount::load(__views[#offset_expr], __program_id)?;
        }
    };

    // --- Constraints ---
    let mut constraints = Vec::new();

    // Writable check is now owned by `AnchorAccount::load_mut` (default
    // impl in `lang-v2/src/traits.rs`), so the derive no longer emits a
    // separate constraint block for `#[account(mut)]`. Types that
    // override `load_mut` (Slab/Account, BorshAccount, Signer, Boxed,
    // Option) each validate is_writable themselves; types that inherit
    // the default (UncheckedAccount, SystemAccount, Program, Sysvar) get
    // it via the trait default.

    // signer check
    if attrs.is_signer {
        constraints.push(quote! {
            if !#field_name.account().is_signer() {
                return Err(anchor_lang_v2::ErrorCode::ConstraintSigner.into());
            }
        });
    }

    // executable check
    if attrs.is_executable {
        constraints.push(quote! {
            if !#field_name.account().executable() {
                return Err(anchor_lang_v2::ErrorCode::ConstraintExecutable.into());
            }
        });
    }

    // rent_exempt check
    if let Some(true) = attrs.rent_exempt {
        constraints.push(quote! {
            if !anchor_lang_v2::is_rent_exempt(#field_name.account()) {
                return Err(anchor_lang_v2::ErrorCode::ConstraintRentExempt.into());
            }
        });
    }

    // Seeds constraint. Runs for all non-init fields, INCLUDING
    // init_if_needed: when the account already exists the init body
    // (which contains its own seeds check) is skipped, so this is the
    // only PDA verification on that path. For plain `init`, the seeds
    // check inside emit_init_body is authoritative and this block is
    // skipped to avoid a redundant find loop.
    if !attrs.is_init {
        if let Some(ref seeds_expr) = attrs.seeds {
            let using_our_program_id = attrs.seeds_program.is_none();
            let pda_program = match &attrs.seeds_program {
                Some(prog) => quote! { &#prog },
                None => quote! { __program_id },
            };
            if let Expr::Array(arr) = seeds_expr {
                // Array-literal seeds: `seeds = [b"vault", user.address().as_ref()]`
                let seed_elems: Vec<&Expr> = arr.elems.iter().collect();
                let rewritten: Vec<_> = seed_elems
                    .iter()
                    .map(|s| rewrite_seed_expr(s, field_names))
                    .collect();
                if let Some(Some(ref bump_expr)) = attrs.bump {
                    let bump_assign = if is_optional {
                        quote! { Some(__bump_val) }
                    } else {
                        quote! { __bump_val }
                    };
                    constraints.push(quote! {
                        {
                            let __bump_val: u8 = #bump_expr;
                            anchor_lang_v2::verify_program_address(
                                &[#(#rewritten),* , &[__bump_val]],
                                #pda_program,
                                #field_name.account().address(),
                            )?;
                            __bumps.#field_name = #bump_assign;
                        }
                    });
                } else {
                    let target_addr_ref = quote! { #field_name.account().address() };
                    constraints.push(emit_seeds_check(
                        &seed_elems,
                        &rewritten,
                        &pda_program,
                        &target_addr_ref,
                        field_name,
                        Some(field_ty),
                        false,
                        using_our_program_id,
                        is_optional,
                    ));
                }
            } else {
                // Opaque expression: `seeds = Counter::seeds()` etc.
                let bump_assign = if is_optional {
                    quote! { Some(__bump) }
                } else {
                    quote! { __bump }
                };
                if let Some(Some(ref bump_expr)) = attrs.bump {
                    // Explicit bump + expression seeds: verify with appended bump
                    constraints.push(quote! {
                        {
                            let __seed_val = #seeds_expr;
                            let __seed_ref: &[&[u8]] = __seed_val.as_ref();
                            let __bump_val: u8 = #bump_expr;
                            let __bump_bytes = [__bump_val];
                            let mut __seed_buf: [&[u8]; 17] = [&[]; 17];
                            let __n = __seed_ref.len();
                            __seed_buf[..__n].copy_from_slice(__seed_ref);
                            __seed_buf[__n] = &__bump_bytes;
                            anchor_lang_v2::verify_program_address(
                                &__seed_buf[..__n + 1],
                                #pda_program,
                                #field_name.account().address(),
                            )?;
                            __bumps.#field_name = __bump_val;
                        }
                    });
                } else {
                    // Bare bump: use find_and_verify with skip_curve
                    // when the account type guarantees non-zero data.
                    let skip_curve = quote! {
                        <#field_ty as anchor_lang_v2::AnchorAccount>::MIN_DATA_LEN > 0
                    };
                    let target_addr = quote! { #field_name.account().address() };
                    constraints.push(quote! {
                        {
                            let __seed_val = #seeds_expr;
                            let __seed_ref: &[&[u8]] = __seed_val.as_ref();
                            let __bump = if #skip_curve {
                                anchor_lang_v2::find_and_verify_program_address_skip_curve(
                                    __seed_ref, #pda_program, #target_addr,
                                ).map_err(|_| anchor_lang_v2::ErrorCode::ConstraintSeeds)?
                            } else {
                                anchor_lang_v2::find_and_verify_program_address(
                                    __seed_ref, #pda_program, #target_addr,
                                ).map_err(|_| anchor_lang_v2::ErrorCode::ConstraintSeeds)?
                            };
                            __bumps.#field_name = #bump_assign;
                        }
                    });
                }
            }
        }
    }

    // has_one
    for (ho, ho_err) in &attrs.has_one {
        let err = if let Some(ref e) = ho_err {
            quote! { core::convert::Into::into(#e) }
        } else {
            quote! { anchor_lang_v2::ErrorCode::ConstraintHasOne.into() }
        };
        constraints.push(quote! {
            if AsRef::<[u8]>::as_ref(&#field_name.#ho) != AsRef::<[u8]>::as_ref(#ho.account().address()) {
                return Err(#err);
            }
        });
    }

    // address
    if let Some(ref addr) = attrs.address {
        let err = if let Some(ref e) = attrs.address_error {
            quote! { core::convert::Into::into(#e) }
        } else {
            quote! { anchor_lang_v2::ErrorCode::ConstraintAddress.into() }
        };
        constraints.push(quote! {
            {
                // Bind the expected address to a local for `address_eq`.
                let __expected: anchor_lang_v2::Address = #addr;
                if !anchor_lang_v2::address_eq(#field_name.account().address(), &__expected) {
                    return Err(#err);
                }
            }
        });
    }

    // owner
    if let Some(ref owner_expr) = attrs.owner {
        let err = if let Some(ref e) = attrs.owner_error {
            quote! { core::convert::Into::into(#e) }
        } else {
            quote! { anchor_lang_v2::ErrorCode::ConstraintOwner.into() }
        };
        constraints.push(quote! {
            if !#field_name.account().owned_by(&#owner_expr) {
                return Err(#err);
            }
        });
    }

    // constraint
    if let Some(ref expr) = attrs.constraint {
        let err = if let Some(ref custom_err) = attrs.constraint_error {
            quote! { core::convert::Into::into(#custom_err) }
        } else {
            quote! { anchor_lang_v2::ErrorCode::ConstraintRaw.into() }
        };
        constraints.push(quote! {
            if !(#expr) {
                return Err(#err);
            }
        });
    }

    // Namespaced constraints: usually skipped on init/init_if_needed
    // because those constraints are instead threaded into
    // `AccountInitialize::Params` at init time. Runtime-only namespaces
    // (see `is_runtime_only_constraint_ns`) still need the `Constrain`
    // call on init paths — they aren't init params.
    for nc in &attrs.namespaced {
        let is_runtime_only = is_runtime_only_constraint_ns(&nc.namespace);
        if (attrs.is_init || attrs.is_init_if_needed) && !is_runtime_only {
            continue;
        }
        let ns = syn::Ident::new(&nc.namespace, proc_macro2::Span::call_site());
        let key = syn::Ident::new(&nc.key, proc_macro2::Span::call_site());
        let value = &nc.value;
        // BYOC: marker path resolves via user's `use` imports. Field
        // refs go through `AsRef::as_ref` with V inferred from the
        // Constrain impl (wrappers impl both `AsRef<Address>` and
        // `AsRef<AccountView>`). Literals pass through as `&value`.
        let expected = if nc.is_field_ref {
            quote! { AsRef::as_ref(&#value) }
        } else {
            quote! { &#value }
        };
        constraints.push(quote! {
            anchor_lang_v2::Constrain::<#ns::#key, _>::constrain(
                &mut #field_name, #expected,
            )?;
        });
    }

    // realloc
    if let Some(ref new_space) = attrs.realloc {
        let realloc_payer = attrs
            .realloc_payer
            .as_ref()
            .expect("realloc requires realloc_payer");
        let zero_fill = attrs.realloc_zero;
        // BorshAccount holds a pinocchio RefMut that (a) blocks the system
        // program Transfer CPI inside realloc_account and (b) captures a
        // stale slice length after resize. Release before, reacquire after.
        // Slab holds no pinocchio borrow so needs neither step.
        let base_ty = option_inner.unwrap_or(field_ty);
        let is_borsh_account = field_ty_str(base_ty) == "BorshAccount";
        let pre_realloc = if is_borsh_account {
            quote! { #field_name.release_borrow(); }
        } else {
            quote! {}
        };
        let post_realloc = if is_borsh_account {
            quote! { #field_name.reacquire_borrow_mut()?; }
        } else {
            quote! {}
        };
        constraints.push(quote! {
            {
                let __new_space = #new_space;
                let mut __view = *#field_name.account();
                let __payer_view = *#realloc_payer.account();
                if __new_space != __view.data_len() {
                    #pre_realloc
                    anchor_lang_v2::realloc_account(
                        &mut __view,
                        __new_space,
                        &__payer_view,
                        #zero_fill,
                    )?;
                    #post_realloc
                }
            }
        });
    }

    // close (self-close prevention constraint + exit)
    let exit = if let Some(ref close_target) = attrs.close {
        constraints.push(quote! {
            if anchor_lang_v2::address_eq(
                #field_name.account().address(),
                #close_target.account().address(),
            ) {
                return Err(anchor_lang_v2::ErrorCode::ConstraintClose.into());
            }
        });
        Some(quote! {
            anchor_lang_v2::AnchorAccount::close(
                &mut self.#field_name,
                *self.#close_target.account(),
            )?;
        })
    } else if attrs.is_mut {
        Some(quote! {
            anchor_lang_v2::AnchorAccount::exit(&mut self.#field_name)?;
        })
    } else {
        None
    };

    // Dup-check emission: stored separately from `constraints` so the
    // struct-level codegen can aggregate all mut fields' dup checks under a
    // single outer `if let Some(__dups) = __duplicates { ... }` gate. That
    // way non-dup txs pay one Option-tag branch regardless of field count.
    let dup_check = if attrs.is_mut && !attrs.is_dup {
        Some(quote! {
            if __dups.get((__base_offset + #offset_expr) as u8) {
                return Err(anchor_lang_v2::ErrorCode::ConstraintDuplicateMutableAccount.into());
            }
        })
    } else {
        None
    };

    // For `Option<T>` fields, each constraint body was generated against the
    // unwrapped inner — we wrap it in `if let Some(#field_name) = #field_name`
    // so `#field_name.account()`, `#field_name.authority`, etc. resolve on the
    // inner `T` (via autoderef). The exit/close path regenerates against the
    // unwrapped `&mut T` so `AnchorAccount::exit/close` get the right type.
    //
    // Mutable fields use `ref mut` so constraint bodies that need `&mut self`
    // (e.g. BorshAccount::release_borrow in the realloc path) can work.
    // Read-only methods still resolve via auto-deref from `&mut T` to `&T`.
    let (constraints, exit) = if is_optional {
        let constraints = constraints
            .into_iter()
            .map(|c| {
                if attrs.is_mut {
                    quote! {
                        if let Some(ref mut #field_name) = #field_name {
                            let _ = &#field_name;
                            #c
                        }
                    }
                } else {
                    quote! {
                        if let Some(ref #field_name) = #field_name {
                            // `#c` may not textually name `#field_name` (e.g. a
                            // literal `constraint = false`, or the derive-
                            // generated duplicate-mut guard that only touches
                            // `__duplicates[..]`). Without this no-op reference
                            // rustc flags the original field as unused. Narrow
                            // silencer rather than a blanket
                            // `#[allow(unused_variables)]` so real typos in
                            // `#c` still surface.
                            let _ = &#field_name;
                            #c
                        }
                    }
                }
            })
            .collect();
        let exit = exit.map(|e| {
            // `e` was built against `self.#field_name` (e.g.
            // `AnchorAccount::exit(&mut self.#field_name)`). For optional
            // fields we rebuild with the unwrapped inner so the trait call
            // dispatches on `T`, not `Option<T>`.
            //
            // The content of `e` is a fixed shape (either `close(...)?;` or
            // `exit(...)?;`), so we don't need to parse/rewrite — we just
            // regenerate from scratch based on which attr set it.
            let _ = e; // silence unused (shape decided below)
            if let Some(ref close_target) = attrs.close {
                quote! {
                    if let Some(__inner) = self.#field_name.as_mut() {
                        anchor_lang_v2::AnchorAccount::close(
                            __inner,
                            *self.#close_target.account(),
                        )?;
                    }
                }
            } else {
                quote! {
                    if let Some(__inner) = self.#field_name.as_mut() {
                        anchor_lang_v2::AnchorAccount::exit(__inner)?;
                    }
                }
            }
        });
        (constraints, exit)
    } else {
        (constraints, exit)
    };

    Ok(AccountField {
        name: field_name.clone(),
        ty: field.ty.clone(),
        load,
        constraints,
        dup_check,
        exit,
        has_bump,
        is_optional,
        idl_writable,
        idl_init_signer,
        idl_has_one,
        idl_docs,
        idl_pda,
        idl_field_ty,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_account_attrs() {
        let attrs: Vec<Attribute> = vec![syn::parse_quote!(
            #[account(mut, seeds = [b"vault"], bump, signer)]
        )];
        let parsed_attrs = parse_account_attrs(&attrs).unwrap();
        assert!(parsed_attrs.is_mut);
        assert!(parsed_attrs.seeds.is_some());
        assert!(parsed_attrs.bump.is_some());
        assert!(parsed_attrs.is_signer);
    }

    #[test]
    fn test_parse_invalid_account_attrs() {
        let attrs: Vec<Attribute> = vec![syn::parse_quote!(
            #[account(mut, seeds = [b"vault"], bumpp, signer)]
        )];

        let err = match parse_account_attrs(&attrs) {
            Ok(_) => panic!("expected malformed account attrs to be rejected"),
            Err(err) => err,
        };

        assert!(
            err.to_string()
                .contains("unknown account constraint `bumpp`"),
            "unexpected error: {err}"
        );
    }
}
