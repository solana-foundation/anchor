use {
    proc_macro2::TokenStream as TokenStream2,
    quote::quote,
    syn::{
        ext::IdentExt,
        parse::{Parse, ParseStream},
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
    pub seeds: Option<Vec<Expr>>,
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

pub fn parse_account_attrs(attrs: &[Attribute]) -> AccountAttrs {
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
        let _ = attr.parse_args_with(|input: ParseStream| {
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
                        let content;
                        syn::bracketed!(content in input);
                        let seeds = content
                            .parse_terminated(Expr::parse, Token![,])?
                            .into_iter()
                            .collect();
                        result.seeds = Some(seeds);
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
        });
    }
    result
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
    seeds: &[Expr],
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
    let skip_curve = if for_init {
        quote! { true }
    } else if let Some(ty) = field_ty {
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
    // inputs (e.g. `mint::Authority = x`). Runtime-only constraints —
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

    let seeds_arg = if let Some(ref seeds) = attrs.seeds {
        let seed_exprs: Vec<_> = seeds
            .iter()
            .map(|s| rewrite_seed_expr(s, field_names))
            .collect();
        let using_our_program_id = attrs.seeds_program.is_none();
        let pda_program = match &attrs.seeds_program {
            Some(prog) => quote! { &#prog },
            None => quote! { __program_id },
        };
        emit_seeds_check(
            seeds,
            &seed_exprs,
            &pda_program,
            &quote! { __target.address() },
            field_name,
            None,
            true,
            using_our_program_id,
            is_optional,
        )
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

pub fn parse_field(field: &syn::Field, field_names: &[String], field_index: u8) -> AccountField {
    let field_index_usize = field_index as usize;
    let field_name = field.ident.as_ref().expect("named field");
    let field_ty = &field.ty;
    let attrs = parse_account_attrs(&field.attrs);

    let option_inner = extract_option_inner(field_ty);
    let is_optional = option_inner.is_some();
    // Fresh-keypair init (no seeds) — caller signs the tx. Distinct from
    // `Signer`-type fields, which the IDL picks up through
    // `IdlAccountType::__IDL_IS_SIGNER` at runtime.
    let idl_init_signer = (attrs.is_init || attrs.is_init_if_needed) && attrs.seeds.is_none();
    let idl_writable = attrs.is_mut;
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
    let load = if is_nested_type(field_ty) {
        quote! { compile_error!("Nested<T> codegen not yet implemented"); }
    } else if let Some(inner_ty) = option_inner {
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
                let __target = __views[#field_index_usize];
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
                let __target = __views[#field_index_usize];
                #init_body
            };
        }
    } else if attrs.is_init_if_needed {
        let init_body = emit_init_body(field_name, field_ty, &attrs, field_names, false);
        quote! {
            let mut #field_name: #field_ty = {
                let __target = __views[#field_index_usize];
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
                let __target = __views[#field_index_usize];
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
            let mut #field_name = unsafe { <#field_ty as anchor_lang_v2::AnchorAccount>::load_mut(__views[#field_index_usize], __program_id)? };
        }
    } else {
        quote! {
            let #field_name: #field_ty = anchor_lang_v2::AnchorAccount::load(__views[#field_index_usize], __program_id)?;
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
        if let Some(ref seeds) = attrs.seeds {
            let seed_exprs: Vec<_> = seeds
                .iter()
                .map(|s| rewrite_seed_expr(s, field_names))
                .collect();
            // seeds::program = expr overrides which program_id to derive PDA from
            let using_our_program_id = attrs.seeds_program.is_none();
            let pda_program = match &attrs.seeds_program {
                Some(prog) => quote! { &#prog },
                None => quote! { __program_id },
            };
            if let Some(Some(ref bump_expr)) = attrs.bump {
                // User-supplied bump (e.g. stored in account data). Always
                // User-supplied bump — verify directly.
                let bump_assign = if is_optional {
                    quote! { Some(__bump_val) }
                } else {
                    quote! { __bump_val }
                };
                constraints.push(quote! {
                    {
                        let __bump_val: u8 = #bump_expr;
                        anchor_lang_v2::verify_program_address(
                            &[#(#seed_exprs),* , &[__bump_val]],
                            #pda_program,
                            #field_name.account().address(),
                        )?;
                        __bumps.#field_name = #bump_assign;
                    }
                });
            } else {
                let target_addr_ref = quote! { #field_name.account().address() };
                constraints.push(emit_seeds_check(
                    seeds,
                    &seed_exprs,
                    &pda_program,
                    &target_addr_ref,
                    field_name,
                    Some(field_ty),
                    /* for_init */ false,
                    using_our_program_id,
                    is_optional,
                ));
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
        constraints.push(quote! {
            {
                let __new_space = #new_space;
                let mut __view = *#field_name.account();
                let __payer_view = *#realloc_payer.account();
                if __new_space != __view.data_len() {
                    // Slab holds no pinocchio borrow, so resize() proceeds
                    // without any release/reacquire dance.
                    anchor_lang_v2::realloc_account(
                        &mut __view,
                        __new_space,
                        &__payer_view,
                        #zero_fill,
                    )?;
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

    if attrs.is_mut && !attrs.is_dup {
        constraints.push(quote! {
            if __duplicates.get(#field_index) {
                return Err(anchor_lang_v2::ErrorCode::ConstraintDuplicateMutableAccount.into());
            }
        });
    }

    // For `Option<T>` fields, each constraint body was generated against the
    // unwrapped inner — we wrap it in `if let Some(#field_name) = #field_name`
    // so `#field_name.account()`, `#field_name.authority`, etc. resolve on the
    // inner `T` (via autoderef). The exit/close path regenerates against the
    // unwrapped `&mut T` so `AnchorAccount::exit/close` get the right type.
    let (constraints, exit) = if is_optional {
        let constraints = constraints
            .into_iter()
            .map(|c| {
                quote! {
                    if let Some(ref #field_name) = #field_name {
                        #c
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

    AccountField {
        name: field_name.clone(),
        ty: field.ty.clone(),
        load,
        constraints,
        exit,
        has_bump,
        is_optional,
        idl_writable,
        idl_init_signer,
        idl_field_ty,
    }
}
