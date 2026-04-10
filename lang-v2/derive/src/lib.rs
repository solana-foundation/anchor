extern crate proc_macro;

mod idl;
mod parse;

use {
    proc_macro::TokenStream,
    proc_macro2::TokenStream as TokenStream2,
    quote::quote,
    syn::{parse_macro_input, Data, DeriveInput, Fields, FnArg, ItemMod, Pat, Type},
    parse::{parse_account_attrs, field_ty_str, is_nested_type},
};

// ---------------------------------------------------------------------------
// #[derive(Accounts)]
// ---------------------------------------------------------------------------

#[proc_macro_derive(Accounts, attributes(account))]
pub fn derive_accounts(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(impl_accounts(&input))
}

fn impl_accounts(input: &DeriveInput) -> TokenStream2 {
    let name = &input.ident;
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Accounts derive only supports named fields"),
        },
        _ => panic!("Accounts derive only supports structs"),
    };

    let mut load_stmts = Vec::new();
    let mut constraint_stmts = Vec::new();
    let mut exit_stmts = Vec::new();
    let mut field_names = Vec::new();
    let mut idl_accounts: Vec<(String, bool, bool)> = Vec::new();

    let account_count = fields.iter().filter(|f| !is_nested_type(&f.ty)).count();

    for field in fields.iter() {
        let field_name = field.ident.as_ref().expect("named field");
        let field_ty = &field.ty;
        let attrs = parse_account_attrs(&field.attrs);
        let name_str = field_name.to_string();

        field_names.push(field_name.clone());

        let is_signer = field_ty_str(field_ty) == "Signer";
        idl_accounts.push((name_str.clone(), attrs.is_mut, is_signer || attrs.is_init));

        if is_nested_type(field_ty) {
            load_stmts.push(quote! { compile_error!("Nested<T> codegen not yet implemented"); });
            continue;
        }

        // --- Init ---
        // TODO: move init logic into creator structs (SystemCreate, TokenAccountCreate, etc.)
        // so it's testable library code instead of codegen. The macro would construct the
        // appropriate creator and call .create(), then T::init(). See plan.md.
        if attrs.is_init {
            let payer = attrs.payer.as_ref().expect("#[account(init)] requires payer");
            let space = attrs.space.as_ref().expect("#[account(init)] requires space");

            let create_call = if let Some(ref seeds) = attrs.seeds {
                let seed_exprs = seeds;
                quote! {
                    let (__pda, __bump) = anchor_lang_v2::find_program_address(
                        &[#(#seed_exprs),*], __program_id,
                    );
                    if *__target.address() != __pda {
                        return Err(anchor_lang_v2::ErrorCode::ConstraintSeeds.into());
                    }
                    anchor_lang_v2::create_account_signed(
                        __payer, &__target, #space, __program_id,
                        &[#(#seed_exprs),* , &[__bump]],
                    )?;
                }
            } else {
                quote! {
                    anchor_lang_v2::create_account(__payer, &__target, #space, __program_id)?;
                }
            };

            load_stmts.push(quote! {
                let mut #field_name = {
                    let __target = __accounts[__account_idx];
                    let __payer = #payer.account();
                    #create_call
                    <#field_ty as anchor_lang_v2::AnchorAccountInit>::init(__target, __program_id)?
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

        // --- Seeds constraint (non-init) ---
        if !attrs.is_init {
            if let Some(ref seeds) = attrs.seeds {
                let seed_exprs = seeds;
                constraint_stmts.push(quote! {
                    let (__pda, _) = anchor_lang_v2::find_program_address(
                        &[#(#seed_exprs),*], __program_id,
                    );
                    if *#field_name.account().address() != __pda {
                        return Err(anchor_lang_v2::ErrorCode::ConstraintSeeds.into());
                    }
                });
            }
        }

        // --- has_one ---
        for ho in &attrs.has_one {
            constraint_stmts.push(quote! {
                if AsRef::<[u8]>::as_ref(&#field_name.#ho) != AsRef::<[u8]>::as_ref(#ho.account().address()) {
                    return Err(anchor_lang_v2::ErrorCode::ConstraintHasOne.into());
                }
            });
        }

        // --- address ---
        if let Some(ref addr) = attrs.address {
            constraint_stmts.push(quote! {
                if *#field_name.account().address() != #addr {
                    return Err(anchor_lang_v2::ErrorCode::ConstraintAddress.into());
                }
            });
        }

        // --- exit / close ---
        if let Some(ref close_target) = attrs.close {
            exit_stmts.push(quote! {
                anchor_lang_v2::AnchorAccount::close(&mut self.#field_name, *self.#close_target.account())?;
            });
        } else if attrs.is_mut {
            exit_stmts.push(quote! {
                anchor_lang_v2::AnchorAccount::exit(&mut self.#field_name)?;
            });
        }
    }

    let idl_json = idl::build_accounts_json(&idl_accounts);

    quote! {
        impl #name {
            #[cfg(feature = "idl-build")]
            pub const __IDL_ACCOUNTS: &'static str = #idl_json;

            pub fn try_accounts(
                __program_id: &anchor_lang_v2::Address,
                __accounts: &[anchor_lang_v2::AccountView],
            ) -> anchor_lang_v2::Result<(Self, usize)> {
                use anchor_lang_v2::AnchorAccount as _;
                if __accounts.len() < #account_count {
                    return Err(anchor_lang_v2::ErrorCode::AccountNotEnoughKeys.into());
                }
                let mut __account_idx: usize = 0;
                #(#load_stmts)*
                #(#constraint_stmts)*
                Ok((Self { #(#field_names),* }, __account_idx))
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
                #[derive(anchor_lang_v2::AnchorDeserialize)]
                struct __Args { #(#extra_arg_names: #extra_arg_types,)* }
                let __args = <__Args as anchor_lang_v2::AnchorDeserialize>::deserialize(
                    &mut &__ix_data[..]
                ).map_err(|_| anchor_lang_v2::ErrorCode::InstructionDidNotDeserialize)?;
                #(let #extra_arg_names = __args.#extra_arg_names;)*
            }
        };

        idl_ix_names.push(fn_name_str.clone());
        idl_ix_discs.push(idl::disc_json(disc_bytes));
        idl_ix_args.push(idl::build_args_json(&extra_args));
        idl_accounts_types.push(accounts_type.clone());

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
                let (__ctx_accounts, __consumed) = #accounts_type::try_accounts(__program_id, __accounts)?;
                let __remaining = &__accounts[__consumed..];
                let mut __ctx = anchor_lang_v2::Context::new(*__program_id, __ctx_accounts, __remaining);
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

        #[cfg(feature = "idl-build")]
        pub fn __build_idl_instructions() -> Vec<String> {
            vec![
                #(
                    format!(
                        "{{\"name\":\"{}\",\"discriminator\":{},\"accounts\":{},\"args\":{}}}",
                        #idl_ix_names,
                        #idl_ix_discs,
                        #idl_accounts_types::__IDL_ACCOUNTS,
                        #idl_ix_args,
                    )
                ),*
            ]
        }
    }
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
