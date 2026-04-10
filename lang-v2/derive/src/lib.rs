extern crate proc_macro;

mod idl;
mod parse;

use {
    proc_macro::TokenStream,
    proc_macro2::TokenStream as TokenStream2,
    quote::quote,
    syn::{parse_macro_input, Data, DeriveInput, Fields, FnArg, Ident, ItemMod, Pat, Type},
    parse::{parse_account_attrs, field_ty_str, is_nested_type, extract_inner_data_type, extract_inner_type_for_init, NamespacedConstraint},
};

/// Generate PDA seeds derivation + validation + optional signer seeds.
/// Used by init, init_if_needed, and non-init seeds constraints.
fn gen_init_seeds(
    seeds: &[syn::Expr],
    field_name: &Ident,
) -> TokenStream2 {
    quote! {
        let (__pda, __bump) = anchor_lang_v2::find_program_address(
            &[#(#seeds),*], __program_id,
        );
        if *__target.address() != __pda {
            return Err(anchor_lang_v2::ErrorCode::ConstraintSeeds.into());
        }
        __bumps.#field_name = __bump;
        let __seeds: Option<&[&[u8]]> = Some(&[#(#seeds),* , &[__bump]]);
    }
}

/// Build init param assignments from namespaced constraints.
fn gen_init_params(namespaced: &[NamespacedConstraint]) -> Vec<TokenStream2> {
    namespaced.iter().map(|nc| {
        let key = Ident::new(
            &nc.key.to_lowercase(),
            proc_macro2::Span::call_site(),
        );
        let value = &nc.value;
        if nc.is_field_ref {
            quote! { __p.#key = Some(#value.account()); }
        } else {
            quote! { __p.#key = Some(#value); }
        }
    }).collect()
}

// ---------------------------------------------------------------------------
// #[derive(Accounts)]
// ---------------------------------------------------------------------------

#[proc_macro_derive(Accounts, attributes(account, instruction))]
pub fn derive_accounts(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(impl_accounts(&input))
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
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Accounts derive only supports named fields"),
        },
        _ => panic!("Accounts derive only supports structs"),
    };

    // Parse #[instruction(arg: Type, ...)] for early deserialization
    let ix_args = parse_instruction_attrs(&input.attrs);

    let mut load_stmts = Vec::new();
    let mut constraint_stmts = Vec::new();
    let mut exit_stmts = Vec::new();
    let mut field_names = Vec::new();
    // (name, writable, signer, optional_program_address)
    let mut idl_accounts: Vec<(String, bool, bool, Option<String>)> = Vec::new();
    let mut idl_data_types: Vec<TokenStream2> = Vec::new(); // inner T from BorshAccount<T>/Account<T>

    // Bumps: collect fields that have seeds (init with seeds, init_if_needed with seeds,
    // or non-init with seeds constraint). Each gets a u8 entry in the bumps struct.
    let mut bump_field_names: Vec<syn::Ident> = Vec::new();

    let account_count = fields.iter().filter(|f| !is_nested_type(&f.ty)).count();

    for field in fields.iter() {
        let field_name = field.ident.as_ref().expect("named field");
        let field_ty = &field.ty;
        let attrs = parse_account_attrs(&field.attrs);
        let name_str = field_name.to_string();

        field_names.push(field_name.clone());

        let is_signer = field_ty_str(field_ty) == "Signer";
        // init/init_if_needed accounts are only signers if they are NOT PDAs (no seeds)
        let is_init_signer = (attrs.is_init || attrs.is_init_if_needed) && attrs.seeds.is_none();
        let program_address = parse::extract_program_address(field_ty);
        idl_accounts.push((name_str.clone(), attrs.is_mut, is_signer || is_init_signer, program_address));

        // Extract inner data type T from BorshAccount<T> or Account<T> for IDL
        if let Some(inner) = extract_inner_data_type(field_ty) {
            idl_data_types.push(inner);
        }

        // Track whether this field needs a bump entry
        let has_seeds = attrs.seeds.is_some();
        if has_seeds {
            bump_field_names.push(field_name.clone());
        }

        if is_nested_type(field_ty) {
            load_stmts.push(quote! { compile_error!("Nested<T> codegen not yet implemented"); });
            continue;
        }

        // --- Init ---
        if attrs.is_init {
            let payer = attrs.payer.as_ref().expect("#[account(init)] requires payer");
            let space = attrs.space.as_ref()
                .expect("#[account(init)] requires space");
            {
            let inner_ty = extract_inner_type_for_init(field_ty)
                .expect("#[account(init)] requires Account<T> or BorshAccount<T>");
            let param_assignments = gen_init_params(&attrs.namespaced);
            let seeds_arg = if let Some(ref seeds) = attrs.seeds {
                gen_init_seeds(seeds, field_name)
            } else {
                quote! { let __seeds: Option<&[&[u8]]> = None; }
            };

            load_stmts.push(quote! {
                let mut #field_name = {
                    let __target = __accounts[__account_idx];
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
                    <#field_ty as anchor_lang_v2::AnchorAccount>::load_mut(__target, __program_id)?
                };
                __account_idx += 1;
            });
            }
        } else if attrs.is_init_if_needed {
            // --- init_if_needed ---
            // Same as init, but skips creation if already initialized.
            let payer = attrs.payer.as_ref().expect("#[account(init_if_needed)] requires payer");
            let space = attrs.space.as_ref()
                .expect("#[account(init_if_needed)] requires space");
            let inner_ty = extract_inner_type_for_init(field_ty)
                .expect("#[account(init_if_needed)] requires Account<T> or BorshAccount<T>");
            let param_assignments = gen_init_params(&attrs.namespaced);
            let seeds_arg = if let Some(ref seeds) = attrs.seeds {
                gen_init_seeds(seeds, field_name)
            } else {
                quote! { let __seeds: Option<&[&[u8]]> = None; }
            };

            load_stmts.push(quote! {
                let mut #field_name = {
                    let __target = __accounts[__account_idx];
                    let __already_init = __target.owned_by(__program_id) && __target.data_len() > 0;
                    if __already_init {
                        <#field_ty as anchor_lang_v2::AnchorAccount>::load_mut(__target, __program_id)?
                    } else {
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
                        <#field_ty as anchor_lang_v2::AnchorAccount>::load_mut(__target, __program_id)?
                    }
                };
                __account_idx += 1;
            });
        } else {
            // --- Normal load ---
            let load_fn = if attrs.is_mut { quote!(load_mut) } else { quote!(load) };
            let binding = if attrs.is_mut { quote!(let mut) } else { quote!(let) };
            load_stmts.push(quote! {
                #binding #field_name = <#field_ty as anchor_lang_v2::AnchorAccount>::#load_fn(
                    __accounts[__account_idx], __program_id,
                )?;
                __account_idx += 1;
            });
        }

        // --- mut writability check ---
        if attrs.is_mut && !attrs.is_init && !attrs.is_init_if_needed {
            constraint_stmts.push(quote! {
                if !#field_name.account().is_writable() {
                    return Err(anchor_lang_v2::ErrorCode::ConstraintMut.into());
                }
            });
        }

        // --- signer check (explicit #[account(signer)] on non-Signer types) ---
        if attrs.is_signer {
            constraint_stmts.push(quote! {
                if !#field_name.account().is_signer() {
                    return Err(anchor_lang_v2::ErrorCode::ConstraintSigner.into());
                }
            });
        }

        // --- Seeds constraint (non-init, non-init_if_needed) ---
        if !attrs.is_init && !attrs.is_init_if_needed {
            if let Some(ref seeds) = attrs.seeds {
                let seed_exprs = seeds;
                // When bump = <expr> is provided, use verify_program_address
                // (sha256 only, ~200 CU) instead of find_program_address
                // (sha256 + curve per bump, ~544 CU). Skips the curve check
                // since the bump was already validated during account creation.
                if let Some(Some(ref bump_expr)) = attrs.bump {
                    constraint_stmts.push(quote! {
                        {
                            let __bump_val: u8 = #bump_expr;
                            anchor_lang_v2::verify_program_address(
                                &[#(#seed_exprs),* , &[__bump_val]],
                                __program_id,
                                #field_name.account().address(),
                            )?;
                            __bumps.#field_name = __bump_val;
                        }
                    });
                } else {
                    constraint_stmts.push(quote! {
                        let (__pda, __bump) = anchor_lang_v2::find_program_address(
                            &[#(#seed_exprs),*], __program_id,
                        );
                        if *#field_name.account().address() != __pda {
                            return Err(anchor_lang_v2::ErrorCode::ConstraintSeeds.into());
                        }
                        __bumps.#field_name = __bump;
                    });
                }
            }
        }

        // --- has_one ---
        for (ho, ho_err) in &attrs.has_one {
            let err = if let Some(ref e) = ho_err {
                quote! { core::convert::Into::into(#e) }
            } else {
                quote! { anchor_lang_v2::ErrorCode::ConstraintHasOne.into() }
            };
            constraint_stmts.push(quote! {
                if AsRef::<[u8]>::as_ref(&#field_name.#ho) != AsRef::<[u8]>::as_ref(#ho.account().address()) {
                    return Err(#err);
                }
            });
        }

        // --- address ---
        if let Some(ref addr) = attrs.address {
            let err = if let Some(ref e) = attrs.address_error {
                quote! { core::convert::Into::into(#e) }
            } else {
                quote! { anchor_lang_v2::ErrorCode::ConstraintAddress.into() }
            };
            constraint_stmts.push(quote! {
                if *#field_name.account().address() != #addr {
                    return Err(#err);
                }
            });
        }

        // --- owner ---
        if let Some(ref owner_expr) = attrs.owner {
            let err = if let Some(ref e) = attrs.owner_error {
                quote! { core::convert::Into::into(#e) }
            } else {
                quote! { anchor_lang_v2::ErrorCode::ConstraintOwner.into() }
            };
            constraint_stmts.push(quote! {
                if !#field_name.account().owned_by(&#owner_expr) {
                    return Err(#err);
                }
            });
        }

        // --- constraint ---
        if let Some(ref expr) = attrs.constraint {
            let err = if let Some(ref custom_err) = attrs.constraint_error {
                quote! { core::convert::Into::into(#custom_err) }
            } else {
                quote! { anchor_lang_v2::ErrorCode::ConstraintRaw.into() }
            };
            constraint_stmts.push(quote! {
                if !(#expr) {
                    return Err(#err);
                }
            });
        }

        // --- namespaced constraints (token::mint, mint::authority, etc.) ---
        // Skip for init/init_if_needed accounts: namespaced constraints on init
        // accounts are only used as init parameters (e.g. mint::decimals = 6),
        // not runtime validation. Literal values like `6` can't be converted to
        // &Address, and the account may not be fully initialized when constraints run.
        if !attrs.is_init && !attrs.is_init_if_needed {
            for nc in &attrs.namespaced {
                let ns = syn::Ident::new(&nc.namespace, proc_macro2::Span::call_site());
                let key = syn::Ident::new(&nc.key, proc_macro2::Span::call_site());
                let value = &nc.value;
                constraint_stmts.push(quote! {
                    anchor_lang_v2::constraints::Constrain::<
                        anchor_lang_v2::constraints::#ns::#key
                    >::constrain(
                        &#field_name,
                        AsRef::<anchor_lang_v2::Address>::as_ref(&#value),
                    )?;
                });
            }
        }

        // --- realloc ---
        if let Some(ref new_space) = attrs.realloc {
            let realloc_payer = attrs.realloc_payer.as_ref().expect("realloc requires realloc_payer");
            let zero_fill = attrs.realloc_zero;
            constraint_stmts.push(quote! {
                {
                    let __new_space = #new_space;
                    let __info = #field_name.account();
                    let __current_len = __info.data_len();
                    if __new_space != __current_len {
                        // Resize the account
                        let mut __view = *__info;
                        anchor_lang_v2::realloc_account(
                            &mut __view,
                            __new_space,
                            #realloc_payer.account(),
                            #zero_fill,
                        )?;
                    }
                }
            });
        }

        // --- exit / close ---
        if let Some(ref close_target) = attrs.close {
            // Prevent self-closing: closing an account to itself would zero its data
            // and lamports, effectively destroying the account with no destination.
            constraint_stmts.push(quote! {
                if #field_name.account().address() == #close_target.account().address() {
                    return Err(anchor_lang_v2::ErrorCode::ConstraintClose.into());
                }
            });
            exit_stmts.push(quote! {
                anchor_lang_v2::AnchorAccount::close(&mut self.#field_name, *self.#close_target.account())?;
            });
        } else if attrs.is_mut {
            exit_stmts.push(quote! {
                anchor_lang_v2::AnchorAccount::exit(&mut self.#field_name)?;
            });
        }
    }

    // Generate instruction arg deserialization if #[instruction(...)] is present
    let ix_deser = if ix_args.is_empty() {
        quote! {}
    } else {
        let ix_arg_names: Vec<_> = ix_args.iter().map(|(n, _)| n).collect();
        let ix_arg_types: Vec<_> = ix_args.iter().map(|(_, t)| t).collect();
        quote! {
            #[derive(anchor_lang_v2::wincode::SchemaRead)]
            struct __IxArgs { #(#ix_arg_names: #ix_arg_types,)* }
            let __ix_args: __IxArgs = anchor_lang_v2::wincode::deserialize(__ix_data)
                .map_err(|_| anchor_lang_v2::ErrorCode::InstructionDidNotDeserialize)?;
            #(let #ix_arg_names = __ix_args.#ix_arg_names;)*
        }
    };

    let idl_json = idl::build_accounts_json(&idl_accounts);

    // --- Client-side struct for off-chain usage (tests, CPI, SDK) ---
    let client_mod_name = syn::Ident::new(
        &format!("__client_accounts_{}", name.to_string().to_lowercase()),
        name.span(),
    );
    let client_fields: Vec<_> = field_names.iter().map(|f| {
        quote! { pub #f: anchor_lang_v2::Address }
    }).collect();
    let client_meta_entries: Vec<_> = idl_accounts.iter().map(|(fname, writable, signer, _)| {
        let field_ident = syn::Ident::new(fname, proc_macro2::Span::call_site());
        quote! {
            anchor_lang_v2::AccountMeta {
                address: self.#field_ident,
                is_writable: #writable,
                is_signer: #signer,
            }
        }
    }).collect();

    quote! {
        /// Client-side accounts struct with `Address` fields for off-chain use.
        #[cfg(feature = "cpi")]
        pub mod #client_mod_name {
            extern crate alloc;
            use super::*;
            pub struct #name {
                #(#client_fields,)*
            }
            impl anchor_lang_v2::ToAccountMetas for #name {
                fn to_account_metas(&self, _is_signer: Option<bool>) -> alloc::vec::Vec<anchor_lang_v2::AccountMeta> {
                    alloc::vec![#(#client_meta_entries),*]
                }
            }
        }

        /// Auto-generated bumps struct for PDA fields.
        #[derive(Debug, Default, Clone)]
        pub struct #bumps_name {
            #(pub #bump_field_names: u8,)*
        }

        impl anchor_lang_v2::Bumps for #name {
            type Bumps = #bumps_name;
        }

        impl #name {
            #[cfg(feature = "idl-build")]
            pub const __IDL_ACCOUNTS: &'static str = #idl_json;

            #[cfg(feature = "idl-build")]
            pub fn __idl_types() -> Vec<&'static str> {
                vec![#(#idl_data_types::__IDL_TYPE),*]
            }

            pub fn try_accounts(
                __program_id: &anchor_lang_v2::Address,
                __accounts: &[anchor_lang_v2::AccountView],
                __ix_data: &[u8],
            ) -> anchor_lang_v2::Result<(Self, #bumps_name, usize)> {
                use anchor_lang_v2::AnchorAccount as _;
                if __accounts.len() < #account_count {
                    return Err(anchor_lang_v2::ErrorCode::AccountNotEnoughKeys.into());
                }
                #ix_deser
                let mut __account_idx: usize = 0;
                let mut __bumps = #bumps_name::default();
                #(#load_stmts)*
                #(#constraint_stmts)*
                Ok((Self { #(#field_names),* }, __bumps, __account_idx))
            }

            pub fn exit_accounts(&mut self) -> anchor_lang_v2::Result<()> {
                use anchor_lang_v2::AnchorAccount as _;
                #(#exit_stmts)*
                Ok(())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// #[account]
// ---------------------------------------------------------------------------

#[proc_macro_attribute]
pub fn account(attr: TokenStream, item: TokenStream) -> TokenStream {
    let is_borsh = attr.to_string().contains("borsh");
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();
    let vis = &input.vis;
    let attrs = &input.attrs;
    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("#[account] only supports structs"),
    };

    use sha2::Digest;
    let hash = sha2::Sha256::digest(format!("account:{name_str}").as_bytes());
    let disc_bytes = &hash[..8];
    let disc_literals: Vec<_> = disc_bytes.iter().map(|b| quote! { #b }).collect();

    let idl_type_json = if let Fields::Named(named) = fields {
        idl::build_type_json(&name_str, disc_bytes, &named.named)
    } else {
        idl::build_type_json(&name_str, disc_bytes, &syn::punctuated::Punctuated::new())
    };

    let (struct_attrs, pod_impls) = if is_borsh {
        (quote! { #[derive(borsh::BorshSerialize, borsh::BorshDeserialize, Default)] }, quote! {})
    } else {
        let field_checks: Vec<_> = if let Fields::Named(named) = fields {
            named.named.iter().map(|f| {
                let fty = &f.ty;
                quote! {
                    const _: fn() = || {
                        fn assert_pod<T: anchor_lang_v2::bytemuck::Pod>() {}
                        assert_pod::<#fty>();
                    };
                }
            }).collect()
        } else {
            vec![]
        };
        (
            quote! { #[derive(Clone, Copy)] #[repr(C)] },
            quote! {
                #(#field_checks)*
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
            fn owner() -> anchor_lang_v2::Address { crate::ID }
        }
        impl anchor_lang_v2::Discriminator for #name {
            const DISCRIMINATOR: &'static [u8] = &[#(#disc_literals),*];
        }
        #[cfg(feature = "idl-build")]
        impl #name {
            pub const __IDL_TYPE: &'static str = #idl_type_json;
        }
    })
}

// ---------------------------------------------------------------------------
// #[program]
// ---------------------------------------------------------------------------

#[proc_macro_attribute]
pub fn program(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let module = parse_macro_input!(item as ItemMod);
    TokenStream::from(impl_program(&module))
}

fn impl_program(module: &ItemMod) -> TokenStream2 {
    let mod_name = &module.ident;
    let mod_vis = &module.vis;
    let content = match &module.content {
        Some((_, items)) => items,
        None => panic!("#[program] module must have a body"),
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

    let mut dispatch_arms = Vec::new();
    let mut handler_wrappers = Vec::new();
    let mut idl_ix_names: Vec<String> = Vec::new();
    let mut idl_ix_discs: Vec<String> = Vec::new();
    let mut idl_ix_args: Vec<String> = Vec::new();
    let mut idl_accounts_types: Vec<TokenStream2> = Vec::new();
    let mut instruction_structs: Vec<TokenStream2> = Vec::new();
    let mut accounts_reexports: Vec<TokenStream2> = Vec::new();

    for handler in &handlers {
        let fn_name = &handler.sig.ident;
        let fn_name_str = fn_name.to_string();

        use sha2::Digest;
        let hash = sha2::Sha256::digest(format!("global:{fn_name_str}").as_bytes());
        let disc_bytes = &hash[..8];
        let disc_u64 = u64::from_le_bytes(disc_bytes.try_into().unwrap());
        let fn_name_log = format!("Instruction: {fn_name_str}");

        let mut args_iter = handler.sig.inputs.iter();
        let first_arg = args_iter.next().expect("handler must have a Context parameter");
        let accounts_type = extract_context_inner_type(first_arg);

        let extra_args: Vec<_> = args_iter
            .filter_map(|arg| {
                if let FnArg::Typed(pt) = arg {
                    if let Pat::Ident(pi) = &*pt.pat {
                        return Some((&pi.ident, &pt.ty));
                    }
                }
                None
            })
            .collect();

        let extra_arg_names: Vec<_> = extra_args.iter().map(|(n, _)| *n).collect();
        let extra_arg_types: Vec<_> = extra_args.iter().map(|(_, t)| *t).collect();

        let deser_args = if extra_args.is_empty() {
            quote! {}
        } else {
            quote! {
                // Wincode zerocopy instruction arg deserialization.
                #[derive(anchor_lang_v2::wincode::SchemaRead)]
                struct __Args { #(#extra_arg_names: #extra_arg_types,)* }
                let __args: __Args = anchor_lang_v2::wincode::deserialize(__ix_data)
                    .map_err(|_| anchor_lang_v2::ErrorCode::InstructionDidNotDeserialize)?;
                #(let #extra_arg_names = __args.#extra_arg_names;)*
            }
        };

        idl_ix_names.push(fn_name_str.clone());
        idl_ix_discs.push(idl::disc_json(disc_bytes));
        idl_ix_args.push(idl::build_args_json(&extra_args));
        idl_accounts_types.push(accounts_type.clone());

        // --- Client-side instruction struct ---
        let ix_struct_name = syn::Ident::new(
            &to_camel_case(&fn_name_str),
            fn_name.span(),
        );
        let disc_literal_bytes: Vec<_> = disc_bytes.iter().map(|b| quote! { #b }).collect();
        instruction_structs.push(quote! {
            /// Instruction data struct. `.data()` returns discriminator + wincode-encoded args.
            #[derive(anchor_lang_v2::wincode::SchemaWrite)]
            pub struct #ix_struct_name {
                #(pub #extra_arg_names: #extra_arg_types,)*
            }
            impl anchor_lang_v2::Discriminator for #ix_struct_name {
                const DISCRIMINATOR: &'static [u8] = &[#(#disc_literal_bytes),*];
            }
            impl anchor_lang_v2::InstructionData for #ix_struct_name {
                fn data(&self) -> alloc::vec::Vec<u8> {
                    let mut data = alloc::vec::Vec::with_capacity(256);
                    data.extend_from_slice(Self::DISCRIMINATOR);
                    anchor_lang_v2::wincode::serialize_into(&mut data, self)
                        .expect("instruction serialization failed");
                    data
                }
            }
        });

        // --- Client accounts re-export ---
        let client_mod = syn::Ident::new(
            &format!("__client_accounts_{}", accounts_type.to_string().to_lowercase()),
            fn_name.span(),
        );
        accounts_reexports.push(quote! {
            pub use super::#client_mod::#accounts_type;
        });

        dispatch_arms.push(quote! {
            #disc_u64 => __handlers::#fn_name(__program_id, __accounts, &__data[8..]),
        });

        handler_wrappers.push(quote! {
            #[inline(never)]
            pub fn #fn_name(
                __program_id: &anchor_lang_v2::Address,
                __accounts: &[anchor_lang_v2::AccountView],
                __ix_data: &[u8],
            ) -> anchor_lang_v2::Result<()> {
                #[cfg(not(feature = "no-log-ix-name"))]
                anchor_lang_v2::msg!(#fn_name_log);
                #deser_args
                let (__ctx_accounts, __bumps, __consumed) = #accounts_type::try_accounts(__program_id, __accounts, __ix_data)?;
                let __remaining = &__accounts[__consumed..];
                let mut __ctx = anchor_lang_v2::Context::new(*__program_id, __ctx_accounts, __remaining, __bumps);
                #mod_name::#fn_name(&mut __ctx, #(#extra_arg_names),*)?;
                __ctx.accounts.exit_accounts()?;
                Ok(())
            }
        });
    }

    quote! {
        #mod_vis mod #mod_name {
            #(#other_items)*
            #(#handlers)*
        }

        #[cfg(not(feature = "no-entrypoint"))]
        pinocchio::entrypoint!(entry);

        pub fn entry(
            __program_id: &anchor_lang_v2::Address,
            __accounts: &mut [anchor_lang_v2::AccountView],
            __data: &[u8],
        ) -> pinocchio::ProgramResult {
            __try_entry(__program_id, __accounts, __data).map_err(|e| e.into())
        }

        fn __try_entry(
            __program_id: &anchor_lang_v2::Address,
            __accounts: &[anchor_lang_v2::AccountView],
            __data: &[u8],
        ) -> anchor_lang_v2::Result<()> {
            if *__program_id != crate::ID {
                return Err(anchor_lang_v2::ErrorCode::DeclaredProgramIdMismatch.into());
            }
            if __data.len() < 8 {
                return Err(anchor_lang_v2::ErrorCode::InstructionFallbackNotFound.into());
            }
            let __disc = u64::from_le_bytes(__data[..8].try_into().unwrap());
            match __disc {
                #(#dispatch_arms)*
                _ => Err(anchor_lang_v2::ErrorCode::InstructionFallbackNotFound.into()),
            }
        }

        mod __handlers {
            use super::*;
            use anchor_lang_v2::AnchorAccount as _;
            #(#handler_wrappers)*
        }

        /// Client-side instruction structs for off-chain use.
        #[cfg(feature = "cpi")]
        pub mod instruction {
            extern crate alloc;
            use anchor_lang_v2::Discriminator as _;
            #(#instruction_structs)*
        }

        /// Client-side accounts structs (re-exports) for off-chain use.
        #[cfg(feature = "cpi")]
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
                            "{{\"name\":\"{}\",\"discriminator\":{},\"accounts\":{},\"args\":{}}}",
                            #idl_ix_names,
                            #idl_ix_discs,
                            #idl_accounts_types::__IDL_ACCOUNTS,
                            #idl_ix_args,
                        )
                    ),*
                ];

                // Collect types from all accounts structs, dedup by content
                let mut all_types: Vec<&str> = Vec::new();
                #(all_types.extend(#idl_accounts_types::__idl_types());)*
                all_types.sort();
                all_types.dedup();

                // Split each __IDL_TYPE into accounts entry and types entry
                let mut accounts_entries = Vec::new();
                let mut types_entries = Vec::new();
                for ty in &all_types {
                    // __IDL_TYPE is: {"name":"X","discriminator":[...],"type":{"kind":"struct","fields":[...]}}
                    // Split at ,"type": to get accounts part and types part
                    if let Some(pos) = ty.find(",\"type\":") {
                        let name_disc = &ty[..pos];
                        let type_def = &ty[pos+1..ty.len()-1]; // skip trailing }
                        accounts_entries.push(format!("{}}}", name_disc));
                        // Extract name for the types entry
                        let name = ty.split("\"name\":\"").nth(1).unwrap().split("\"").next().unwrap();
                        types_entries.push(format!("{{\"name\":\"{}\",{}}}", name, type_def));
                    }
                }

                let crate_name = env!("CARGO_CRATE_NAME").replace('-', "_");
                let idl = format!(
                    "{{\"address\":\"\",\"metadata\":{{\"name\":\"{}\",\"version\":\"0.1.0\",\"spec\":\"0.1.0\"}},\"instructions\":[{}],\"accounts\":[{}],\"types\":[{}]}}",
                    crate_name,
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
/// Generates:
/// - `#[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]` on the struct
/// - `impl Discriminator` with discriminator = `sha256("event:StructName")[..8]`
/// - `impl Event` (provides `.data()` which serializes discriminator + borsh data)
///
/// # Example
///
/// ```ignore
/// #[event]
/// pub struct DepositRecorded {
///     pub ledger: Address,
///     pub amount: u64,
/// }
/// ```
#[proc_macro_attribute]
pub fn event(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();
    let vis = &input.vis;
    let attrs = &input.attrs;
    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("#[event] only supports structs"),
    };

    use sha2::Digest;
    let hash = sha2::Sha256::digest(format!("event:{name_str}").as_bytes());
    let disc_bytes = &hash[..8];
    let disc_literals: Vec<_> = disc_bytes.iter().map(|b| quote! { #b }).collect();

    TokenStream::from(quote! {
        #[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
        #(#attrs)*
        #vis struct #name #fields

        impl anchor_lang_v2::Discriminator for #name {
            const DISCRIMINATOR: &'static [u8] = &[#(#disc_literals),*];
        }

        impl anchor_lang_v2::Event for #name {}
    })
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

fn extract_context_inner_type(arg: &FnArg) -> TokenStream2 {
    let ty = match arg {
        FnArg::Typed(pt) => &*pt.ty,
        _ => panic!("first parameter must be ctx: &mut Context<T>"),
    };
    if let Type::Reference(r) = ty { return extract_generic_arg(&r.elem); }
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
    panic!("could not extract generic type from Context<T>");
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
