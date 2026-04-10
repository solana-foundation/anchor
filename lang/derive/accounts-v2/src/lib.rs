extern crate proc_macro;

use {
    proc_macro::TokenStream,
    proc_macro2::TokenStream as TokenStream2,
    quote::quote,
    syn::{
        ext::IdentExt,
        parse::{Parse, ParseStream},
        parse_macro_input, Attribute, Data, DeriveInput, Expr, Fields, FnArg, Ident, ItemMod,
        Pat, Token, Type,
    },
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

    let account_count = fields.iter().filter(|f| !is_nested_type(&f.ty)).count();

    for field in fields.iter() {
        let field_name = field.ident.as_ref().expect("named field");
        let field_ty = &field.ty;
        let attrs = parse_account_attrs(&field.attrs);
        let name_str = field_name.to_string();

        field_names.push(field_name.clone());

        if is_nested_type(field_ty) {
            load_stmts.push(quote! {
                compile_error!("Nested<T> codegen not yet implemented");
            });
            continue;
        }

        // --- Seeds / PDA ---
        if let Some(ref seeds) = attrs.seeds {
            let seed_exprs = seeds;
            if attrs.is_init {
                // init + seeds: find PDA, then create with invoke_signed
                let payer = attrs.payer.as_ref().expect("#[account(init)] requires payer");
                let space = attrs.space.as_ref().expect("#[account(init)] requires space");

                load_stmts.push(quote! {
                    let mut #field_name = {
                        let (__pda, __bump) = anchor_lang::v2::find_program_address(
                            &[#(#seed_exprs),*],
                            __program_id,
                        );
                        let __target = __accounts[__account_idx];
                        if *__target.address() != __pda {
                            return Err(anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintSeeds
                            ).with_account_name(#name_str));
                        }
                        let __payer = #payer.account();
                        anchor_lang::v2::create_account_signed(
                            __payer,
                            &__target,
                            #space,
                            __program_id,
                            &[#(#seed_exprs),* , &[__bump]],
                        )?;
                        <#field_ty as anchor_lang::v2::AnchorAccountInit>::init(
                            __target, __program_id,
                        ).map_err(|e| {
                            anchor_lang::error::Error::from(e).with_account_name(#name_str)
                        })?
                    };
                    __account_idx += 1;
                });
            } else {
                // seeds without init: validate PDA address
                let load_fn = if attrs.is_mut { quote!(load_mut) } else { quote!(load) };
                let binding = if attrs.is_mut { quote!(let mut) } else { quote!(let) };

                load_stmts.push(quote! {
                    #binding #field_name = <#field_ty as anchor_lang::v2::AnchorAccount>::#load_fn(
                        __accounts[__account_idx], __program_id,
                    ).map_err(|e| {
                        anchor_lang::error::Error::from(e).with_account_name(#name_str)
                    })?;
                    __account_idx += 1;
                });

                // TODO: when bump value is provided, use create_program_address instead
                // of find_program_address to save ~1000 CUs
                constraint_stmts.push(quote! {
                    let (__pda, _) = anchor_lang::v2::find_program_address(
                        &[#(#seed_exprs),*], __program_id,
                    );
                    if *#field_name.account().address() != __pda {
                        return Err(anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintSeeds
                        ).with_account_name(#name_str));
                    }
                });
            }
        } else if attrs.is_init {
            // init without seeds
            let payer = attrs.payer.as_ref().expect("#[account(init)] requires payer");
            let space = attrs.space.as_ref().expect("#[account(init)] requires space");

            load_stmts.push(quote! {
                let mut #field_name = {
                    let __target = __accounts[__account_idx];
                    let __payer = #payer.account();
                    anchor_lang::v2::create_account(
                        __payer, &__target, #space, __program_id,
                    )?;
                    <#field_ty as anchor_lang::v2::AnchorAccountInit>::init(
                        __target, __program_id,
                    ).map_err(|e| {
                        anchor_lang::error::Error::from(e).with_account_name(#name_str)
                    })?
                };
                __account_idx += 1;
            });
        } else {
            // Normal load
            let load_fn = if attrs.is_mut { quote!(load_mut) } else { quote!(load) };
            let binding = if attrs.is_mut { quote!(let mut) } else { quote!(let) };

            load_stmts.push(quote! {
                #binding #field_name = <#field_ty as anchor_lang::v2::AnchorAccount>::#load_fn(
                    __accounts[__account_idx], __program_id,
                ).map_err(|e| {
                    anchor_lang::error::Error::from(e).with_account_name(#name_str)
                })?;
                __account_idx += 1;
            });
        }

        // --- has_one ---
        for ho in &attrs.has_one {
            constraint_stmts.push(quote! {
                if #field_name.#ho != #ho.account().address().to_bytes() {
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintHasOne
                    ).with_account_name(#name_str));
                }
            });
        }

        // --- address ---
        if let Some(ref addr) = attrs.address {
            constraint_stmts.push(quote! {
                if *#field_name.account().address() != #addr {
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintAddress
                    ).with_account_name(#name_str));
                }
            });
        }

        // --- close ---
        if let Some(ref close_target) = attrs.close {
            exit_stmts.push(quote! {
                anchor_lang::v2::AnchorAccount::close(
                    &mut self.#field_name,
                    *self.#close_target.account(),
                )?;
            });
        } else if attrs.is_mut {
            exit_stmts.push(quote! {
                anchor_lang::v2::AnchorAccount::exit(&mut self.#field_name)?;
            });
        }
    }

    quote! {
        impl #name {
            pub fn try_accounts(
                __program_id: &anchor_lang::v2::Address,
                __accounts: &[anchor_lang::v2::AccountView],
            ) -> anchor_lang::Result<(Self, usize)> {
                use anchor_lang::v2::AnchorAccount as _;
                if __accounts.len() < #account_count {
                    return Err(anchor_lang::error::ErrorCode::AccountNotEnoughKeys.into());
                }
                let mut __account_idx: usize = 0;
                #(#load_stmts)*
                #(#constraint_stmts)*
                Ok((Self { #(#field_names),* }, __account_idx))
            }

            pub fn exit_accounts(&mut self) -> anchor_lang::Result<()> {
                #(#exit_stmts)*
                Ok(())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Attribute parsing
// ---------------------------------------------------------------------------

struct AccountAttrs {
    is_mut: bool,
    is_init: bool,
    has_bump: bool,
    payer: Option<Ident>,
    space: Option<Expr>,
    seeds: Option<Vec<Expr>>,
    has_one: Vec<Ident>,
    address: Option<Expr>,
    close: Option<Ident>,
}

fn parse_account_attrs(attrs: &[Attribute]) -> AccountAttrs {
    let mut result = AccountAttrs {
        is_mut: false,
        is_init: false,
        has_bump: false,
        payer: None,
        space: None,
        seeds: None,
        has_one: Vec::new(),
        address: None,
        close: None,
    };

    for attr in attrs {
        if !attr.path().is_ident("account") {
            continue;
        }
        // Parse as comma-separated items
        let _ = attr.parse_args_with(|input: ParseStream| {
            while !input.is_empty() {
                let ident = Ident::parse_any(input)?;
                match ident.to_string().as_str() {
                    "mut" => result.is_mut = true,
                    "init" => {
                        result.is_init = true;
                        result.is_mut = true;
                    }
                    "bump" => result.has_bump = true,
                    "signer" => {} // validated by Signer type itself
                    "payer" => {
                        input.parse::<Token![=]>()?;
                        result.payer = Some(input.parse()?);
                    }
                    "space" => {
                        input.parse::<Token![=]>()?;
                        result.space = Some(input.parse()?);
                    }
                    "seeds" => {
                        input.parse::<Token![=]>()?;
                        let content;
                        syn::bracketed!(content in input);
                        let seeds = content
                            .parse_terminated(Expr::parse, Token![,])?
                            .into_iter()
                            .collect();
                        result.seeds = Some(seeds);
                    }
                    "has_one" => {
                        input.parse::<Token![=]>()?;
                        result.has_one.push(input.parse()?);
                    }
                    "address" => {
                        input.parse::<Token![=]>()?;
                        result.address = Some(input.parse()?);
                    }
                    "close" => {
                        input.parse::<Token![=]>()?;
                        result.close = Some(input.parse()?);
                    }
                    _ => {} // ignore unknown attrs for forward compat
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

fn is_nested_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Nested";
        }
    }
    false
}

// ---------------------------------------------------------------------------
// #[derive(AnchorData)]
// ---------------------------------------------------------------------------

#[proc_macro_derive(AnchorData)]
pub fn derive_anchor_data(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    use sha2::Digest;
    let hash = sha2::Sha256::digest(format!("account:{}", name).as_bytes());
    let disc_bytes = &hash[..8];
    let disc_literals: Vec<_> = disc_bytes.iter().map(|b| quote! { #b }).collect();

    TokenStream::from(quote! {
        impl anchor_lang::v2::Owner for #name {
            fn owner() -> anchor_lang::v2::Address { crate::ID }
        }
        impl anchor_lang::v2::Discriminator for #name {
            const DISCRIMINATOR: &'static [u8] = &[#(#disc_literals),*];
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

    for handler in &handlers {
        let fn_name = &handler.sig.ident;
        let fn_name_str = fn_name.to_string();

        use sha2::Digest;
        let hash = sha2::Sha256::digest(format!("global:{}", fn_name_str).as_bytes());
        let disc_bytes = &hash[..8];
        let disc_u64 = u64::from_le_bytes(disc_bytes.try_into().unwrap());
        let fn_name_log = format!("Instruction: {}", fn_name_str);

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
                #[derive(anchor_lang::AnchorDeserialize)]
                struct __Args { #(#extra_arg_names: #extra_arg_types,)* }
                let __args = <__Args as anchor_lang::AnchorDeserialize>::deserialize(
                    &mut &__ix_data[..]
                ).map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
                #(let #extra_arg_names = __args.#extra_arg_names;)*
            }
        };

        dispatch_arms.push(quote! {
            #disc_u64 => __handlers::#fn_name(__program_id, __accounts, &__data[8..]),
        });

        handler_wrappers.push(quote! {
            #[inline(never)]
            pub fn #fn_name(
                __program_id: &anchor_lang::v2::Address,
                __accounts: &[anchor_lang::v2::AccountView],
                __ix_data: &[u8],
            ) -> anchor_lang::Result<()> {
                #[cfg(not(feature = "no-log-ix-name"))]
                anchor_lang::v2::msg!(#fn_name_log);
                #deser_args
                let (__ctx_accounts, __consumed) = #accounts_type::try_accounts(__program_id, __accounts)?;
                let __remaining = &__accounts[__consumed..];
                let mut __ctx = anchor_lang::v2::Context::new(*__program_id, __ctx_accounts, __remaining);
                #mod_name::#fn_name(&mut __ctx, #(#extra_arg_names),*)?;
                __ctx.accounts.exit_accounts()?;
                Ok(())
            }
        });
    }

    quote! {
        #mod_vis mod #mod_name {
            use super::*;
            #(#other_items)*
            #(#handlers)*
        }

        #[cfg(not(feature = "no-entrypoint"))]
        pinocchio::entrypoint!(entry);

        pub fn entry(
            __program_id: &anchor_lang::v2::Address,
            __accounts: &mut [anchor_lang::v2::AccountView],
            __data: &[u8],
        ) -> pinocchio::ProgramResult {
            __try_entry(__program_id, __accounts, __data).map_err(|e| e.into())
        }

        fn __try_entry(
            __program_id: &anchor_lang::v2::Address,
            __accounts: &[anchor_lang::v2::AccountView],
            __data: &[u8],
        ) -> anchor_lang::Result<()> {
            if *__program_id != crate::ID {
                return Err(anchor_lang::error::ErrorCode::DeclaredProgramIdMismatch.into());
            }
            if __data.len() < 8 {
                return Err(anchor_lang::error::ErrorCode::InstructionFallbackNotFound.into());
            }
            let __disc = u64::from_le_bytes(__data[..8].try_into().unwrap());
            match __disc {
                #(#dispatch_arms)*
                _ => Err(anchor_lang::error::ErrorCode::InstructionFallbackNotFound.into()),
            }
        }

        mod __handlers {
            use super::*;
            use anchor_lang::v2::AnchorAccount as _;
            #(#handler_wrappers)*
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
