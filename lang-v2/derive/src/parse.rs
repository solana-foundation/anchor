use syn::{ext::IdentExt, parse::{Parse, ParseStream}, Attribute, Expr, Ident, Token, Type};
use quote::quote;
use proc_macro2::TokenStream as TokenStream2;

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
                    "rent_exempt" => {
                        input.parse::<Token![=]>()?;
                        let val: Ident = input.parse()?;
                        result.rent_exempt = Some(val.to_string() == "enforce");
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
                                if !input.is_empty() { input.parse::<Token![,]>()?; }
                                continue;
                            }
                            input.parse::<Token![=]>()?;
                            // Peek to determine if RHS is a simple ident (field ref)
                            // or a literal/expression (value).
                            let is_field_ref = input.peek(syn::Ident);
                            let value: Expr = input.parse()?;
                            // Capitalize key and append Constraint: "mint" → "MintConstraint"
                            let key = {
                                let s = key_ident.to_string();
                                let mut c = s.chars();
                                let capitalized = match c.next() {
                                    Some(first) => first.to_uppercase().to_string() + c.as_str(),
                                    None => String::new(),
                                };
                                format!("{capitalized}Constraint")
                            };
                            let raw_key = key_ident.to_string();
                            result.namespaced.push(NamespacedConstraint {
                                namespace: ident.to_string(),
                                key,
                                raw_key,
                                value,
                                is_field_ref,
                            });
                        }
                        else {
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

/// Extract the inner `T` from `BorshAccount<T>` or `Account<T>`.
///
/// Skips well-known external types (TokenAccount, Mint) that don't have
/// `__IDL_TYPE` since they aren't defined via `#[account]`.
pub fn extract_inner_data_type(ty: &Type) -> Option<proc_macro2::TokenStream> {
    use quote::quote;
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            let name = seg.ident.to_string();
            if name == "BorshAccount" || name == "Account" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        // Skip external types that don't have #[account]-generated __IDL_TYPE
                        if let Type::Path(inner_tp) = inner {
                            if let Some(inner_seg) = inner_tp.path.segments.last() {
                                let inner_name = inner_seg.ident.to_string();
                                if matches!(inner_name.as_str(), "TokenAccount" | "Mint") {
                                    return None;
                                }
                            }
                        }
                        return Some(quote! { #inner });
                    }
                }
            }
        }
    }
    None
}

/// Extract the inner `T` from `BorshAccount<T>` or `Account<T>` for init codegen.
///
/// Unlike `extract_inner_data_type`, this does NOT skip external types like
/// TokenAccount and Mint, since init needs the type for `AccountInitialize` calls.
pub fn extract_inner_type_for_init(ty: &Type) -> Option<proc_macro2::TokenStream> {
    use quote::quote;
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            let name = seg.ident.to_string();
            if name == "BorshAccount" || name == "Account" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(quote! { #inner });
                    }
                }
            }
        }
    }
    None
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
    // IDL metadata
    pub idl_writable: bool,
    pub idl_signer: bool,
    pub idl_program_address: Option<String>,
    pub idl_data_type: Option<TokenStream2>,
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
fn emit_seeds_check(
    seeds: &[Expr],
    seed_exprs: &[TokenStream2],
    pda_program: &TokenStream2,
    target_addr_ref: &TokenStream2,
    field_name: &Ident,
    field_ty: Option<&Type>,
    for_init: bool,
    using_our_program_id: bool,
) -> TokenStream2 {
    // Try to precompute the bump and PDA at expansion time.
    if using_our_program_id {
        if let Some(literal_seeds) = crate::pda::seeds_as_byte_literals(seeds) {
            if let Some(program_id) = crate::pda::discover_program_id() {
                let seed_slices: Vec<&[u8]> =
                    literal_seeds.iter().map(|s| s.as_slice()).collect();
                if let Some((bump, pda_bytes)) =
                    crate::pda::precompute_pda(&seed_slices, &program_id)
                {
                    // Field-scoped const names keep multiple fields'
                    // bumps + PDAs from colliding, even when two
                    // constraints share an outer scope.
                    let upper = field_name.to_string().to_uppercase();
                    let bump_const =
                        Ident::new(&format!("__{}_BUMP", upper), field_name.span());
                    let pda_const =
                        Ident::new(&format!("__{}_PDA", upper), field_name.span());
                    // Emit the 32-byte PDA as an `Address` const.
                    let pda_bytes_tokens = pda_bytes.iter().map(|b| quote! { #b });
                    let check = quote! {
                        const #bump_const: u8 = #bump;
                        const #pda_const: anchor_lang_v2::Address =
                            anchor_lang_v2::Address::new_from_array([#(#pda_bytes_tokens),*]);
                        if !anchor_lang_v2::address_eq(#target_addr_ref, &#pda_const) {
                            return Err(anchor_lang_v2::ErrorCode::ConstraintSeeds.into());
                        }
                        __bumps.#field_name = #bump_const;
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
        __bumps.#field_name = __bump;
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
) -> TokenStream2 {
    let payer = attrs.payer.as_ref().expect("init requires payer");
    let space = attrs.space.as_ref().expect("init requires space");
    let inner_ty = extract_inner_type_for_init(field_ty)
        .expect("init requires Account<T> or BorshAccount<T>");

    let param_assignments: Vec<_> = attrs.namespaced.iter().map(|nc| {
        let key = Ident::new(&nc.raw_key, proc_macro2::Span::call_site());
        let value = &nc.value;
        if nc.is_field_ref {
            quote! { __p.#key = Some(#value.account()); }
        } else {
            quote! { __p.#key = Some(#value); }
        }
    }).collect();

    let seeds_arg = if let Some(ref seeds) = attrs.seeds {
        let seed_exprs: Vec<_> = seeds.iter()
            .map(|s| rewrite_seed_expr(s, field_names)).collect();
        let using_our_program_id = attrs.seeds_program.is_none();
        let pda_program = match &attrs.seeds_program {
            Some(prog) => quote! { &#prog },
            None => quote! { __program_id },
        };
        emit_seeds_check(
            seeds, &seed_exprs,
            &pda_program,
            &quote! { __target.address() },
            field_name, None, true, using_our_program_id,
        )
    } else {
        quote! { let __seeds: Option<&[&[u8]]> = None; }
    };

    quote! {
        let __payer = #payer.account();
        #seeds_arg
        let __init_params = {
            type __P<'__a> = <#inner_ty as anchor_lang_v2::AccountInitialize>::Params<'__a>;
            let mut __p = <__P as Default>::default();
            #(#param_assignments)*
            __p
        };
        <#inner_ty as anchor_lang_v2::AccountInitialize>::create_and_initialize(
            __payer, &__target, #space, __program_id, &__init_params, __seeds,
        )?;
        anchor_lang_v2::AnchorAccount::load_mut_after_init(__target, __program_id)?
    }
}

pub fn parse_field(field: &syn::Field, field_names: &[String], field_index: usize) -> AccountField {
    let field_name = field.ident.as_ref().expect("named field");
    let field_ty = &field.ty;
    let attrs = parse_account_attrs(&field.attrs);
    // Unsuffixed literal so the expansion shows `__views[0]` instead of
    // `__views[0usize]` — the type is unambiguous from the indexing context.
    let field_index = proc_macro2::Literal::usize_unsuffixed(field_index);

    let is_signer = field_ty_str(field_ty) == "Signer";
    let is_optional = field_ty_str(field_ty) == "Optional";
    let is_init_signer = (attrs.is_init || attrs.is_init_if_needed) && attrs.seeds.is_none();
    let program_address = extract_program_address(field_ty);
    let idl_writable = attrs.is_mut;
    let idl_signer = is_signer || is_init_signer;
    let idl_data_type = extract_inner_data_type(field_ty);

    let has_bump = attrs.seeds.is_some();

    // --- Load ---
    let load = if is_nested_type(field_ty) {
        quote! { compile_error!("Nested<T> codegen not yet implemented"); }
    } else if attrs.is_init {
        let init_body = emit_init_body(field_name, field_ty, &attrs, field_names);
        quote! {
            let mut #field_name: #field_ty = {
                let __target = __views[#field_index];
                #init_body
            };
        }
    } else if attrs.is_init_if_needed {
        let init_body = emit_init_body(field_name, field_ty, &attrs, field_names);
        quote! {
            let mut #field_name: #field_ty = {
                let __target = __views[#field_index];
                if __target.data_len() > 0 && !__target.owned_by(&anchor_lang_v2::programs::System::id()) {
                    anchor_lang_v2::AnchorAccount::load_mut(__target, __program_id)?
                } else {
                    #init_body
                }
            };
        }
    } else if attrs.is_zeroed {
        // zeroed: account exists but discriminator must be all zeros. Load mutably,
        // check disc is zero, then write the real discriminator.
        let inner_ty = extract_inner_type_for_init(field_ty)
            .expect("#[account(zeroed)] requires Account<T> or BorshAccount<T>");
        quote! {
            let mut #field_name: #field_ty = {
                let __target = __views[#field_index];
                {
                    let __data = __target.try_borrow()?;
                    let __disc = <#inner_ty as anchor_lang_v2::Discriminator>::DISCRIMINATOR;
                    if __data.len() < __disc.len() || __data[..__disc.len()].iter().any(|b| *b != 0) {
                        return Err(anchor_lang_v2::ErrorCode::ConstraintZero.into());
                    }
                }
                // Write discriminator then load
                unsafe {
                    let mut __view = __target;
                    let __data = __view.borrow_unchecked_mut();
                    let __disc = <#inner_ty as anchor_lang_v2::Discriminator>::DISCRIMINATOR;
                    __data[..__disc.len()].copy_from_slice(__disc);
                }
                anchor_lang_v2::AnchorAccount::load_mut(__target, __program_id)?
            };
        }
    } else if attrs.is_mut {
        quote! {
            let mut #field_name: #field_ty = anchor_lang_v2::AnchorAccount::load_mut(__views[#field_index], __program_id)?;
        }
    } else {
        quote! {
            let #field_name: #field_ty = anchor_lang_v2::AnchorAccount::load(__views[#field_index], __program_id)?;
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
            let seed_exprs: Vec<_> = seeds.iter().map(|s| rewrite_seed_expr(s, field_names)).collect();
            // seeds::program = expr overrides which program_id to derive PDA from
            let using_our_program_id = attrs.seeds_program.is_none();
            let pda_program = match &attrs.seeds_program {
                Some(prog) => quote! { &#prog },
                None => quote! { __program_id },
            };
            if let Some(Some(ref bump_expr)) = attrs.bump {
                // User-supplied bump (e.g. stored in account data). Always
                // User-supplied bump — verify directly.
                constraints.push(quote! {
                    {
                        let __bump_val: u8 = #bump_expr;
                        anchor_lang_v2::verify_program_address(
                            &[#(#seed_exprs),* , &[__bump_val]],
                            #pda_program,
                            #field_name.account().address(),
                        )?;
                        __bumps.#field_name = __bump_val;
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

    // namespaced constraints (skip for init/init_if_needed)
    if !attrs.is_init && !attrs.is_init_if_needed {
        for nc in &attrs.namespaced {
            let ns = syn::Ident::new(&nc.namespace, proc_macro2::Span::call_site());
            let key = syn::Ident::new(&nc.key, proc_macro2::Span::call_site());
            let value = &nc.value;
            // BYOC: marker path resolves via user's `use` imports.
            // Field refs (is_field_ref=true) → convert to Address via AsRef.
            // Literals (is_field_ref=false) → pass directly (e.g. mint::Decimals = 6).
            let expected = if nc.is_field_ref {
                quote! { AsRef::<anchor_lang_v2::Address>::as_ref(&#value) }
            } else {
                quote! { &#value }
            };
            constraints.push(quote! {
                anchor_lang_v2::Constrain::<#ns::#key, _>::constrain(
                    &#field_name, #expected,
                )?;
            });
        }
    }

    // realloc
    if let Some(ref new_space) = attrs.realloc {
        let realloc_payer = attrs.realloc_payer.as_ref().expect("realloc requires realloc_payer");
        let zero_fill = attrs.realloc_zero;
        constraints.push(quote! {
            {
                let __new_space = #new_space;
                // Copy the view out before mutating #field_name so we don't
                // hold an immutable borrow across the release_borrow() call.
                // AccountView is Copy.
                let mut __view = *#field_name.account();
                let __payer_view = *#realloc_payer.account();
                if __new_space != __view.data_len() {
                    // load_mut holds a RefMut on the data buffer; pinocchio's
                    // resize() calls check_borrow_mut() which would see our
                    // outstanding borrow and fail. Drop it, resize, then
                    // reacquire a fresh RefMut against the new buffer.
                    #field_name.release_borrow();
                    anchor_lang_v2::realloc_account(
                        &mut __view,
                        __new_space,
                        &__payer_view,
                        #zero_fill,
                    )?;
                    #field_name.reacquire_borrow_mut()?;
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

    // For Optional<T> fields, wrap all constraints and exit in is_some()
    // guards. When the account is None (client passed the program ID
    // sentinel), code that calls .account() would panic. Skipping is
    // correct: a None optional has no account to validate or close.
    let (constraints, exit) = if is_optional {
        let constraints = constraints.into_iter().map(|c| {
            quote! { if #field_name.is_some() { #c } }
        }).collect();
        let exit = exit.map(|e| {
            quote! { if self.#field_name.is_some() { #e } }
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
        idl_writable,
        idl_signer,
        idl_program_address: program_address,
        idl_data_type,
    }
}

/// Extract the well-known address from `Program<T>` types.
/// Returns the base58 address string for known program types (System, Token, etc.).
pub fn extract_program_address(ty: &Type) -> Option<String> {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Program" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(Type::Path(inner_tp))) = args.args.first() {
                        if let Some(inner_seg) = inner_tp.path.segments.last() {
                            return match inner_seg.ident.to_string().as_str() {
                                "System" => Some("11111111111111111111111111111111".to_string()),
                                "Token" => Some("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string()),
                                "Token2022" => Some("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb".to_string()),
                                "AssociatedToken" => Some("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL".to_string()),
                                "Memo" => Some("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr".to_string()),
                                _ => None,
                            };
                        }
                    }
                }
            }
        }
    }
    None
}
