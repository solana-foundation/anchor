extern crate proc_macro;

mod idl;
mod parse;

use {
    proc_macro::TokenStream,
    proc_macro2::TokenStream as TokenStream2,
    quote::quote,
    syn::{parse_macro_input, Data, DeriveInput, Fields, FnArg, ItemMod, Pat, Type},
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
    let bumps_name = syn::Ident::new(&format!("{name}Bumps"), name.span());
    let fields: Vec<parse::AccountField> = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => named.named.iter().map(parse::parse_field).collect(),
            _ => panic!("Accounts derive only supports named fields"),
        },
        _ => panic!("Accounts derive only supports structs"),
    };

    let field_names: Vec<_> = fields.iter().map(|f| &f.name).collect();
    let loads: Vec<_> = fields.iter().map(|f| &f.load).collect();
    let constraints: Vec<_> = fields.iter().flat_map(|f| &f.constraints).collect();
    let exits: Vec<_> = fields.iter().filter_map(|f| f.exit.as_ref()).collect();
    let bump_fields: Vec<_> = fields.iter().filter(|f| f.has_bump).map(|f| &f.name).collect();

    // IDL collection
    let idl_accounts: Vec<_> = fields.iter().map(|f| {
        (f.name.to_string(), f.idl_writable, f.idl_signer, f.idl_program_address.clone())
    }).collect();
    let idl_json = idl::build_accounts_json(&idl_accounts);
    let idl_data_types: Vec<_> = fields.iter().filter_map(|f| f.idl_data_type.as_ref()).collect();

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
            #(pub #bump_fields: u8,)*
        }

        impl anchor_lang_v2::Bumps for #name {
            type Bumps = #bumps_name;
        }

        impl anchor_lang_v2::TryAccounts for #name {
            fn try_accounts(
                __program_id: &anchor_lang_v2::Address,
                __accounts: &[anchor_lang_v2::AccountView],
            ) -> anchor_lang_v2::Result<(Self, #bumps_name, usize)> {
                use anchor_lang_v2::AnchorAccount as _;
                let mut __loader = anchor_lang_v2::AccountLoader::new(__program_id, __accounts);
                let mut __bumps = #bumps_name::default();
                #(#loads)*
                #(#constraints)*
                Ok((Self { #(#field_names),* }, __bumps, __loader.consumed()))
            }

            fn exit_accounts(&mut self) -> anchor_lang_v2::Result<()> {
                use anchor_lang_v2::AnchorAccount as _;
                #(#exits)*
                Ok(())
            }
        }

        #[cfg(feature = "idl-build")]
        impl #name {
            pub const __IDL_ACCOUNTS: &'static str = #idl_json;

            pub fn __idl_types() -> Vec<&'static str> {
                vec![#(#idl_data_types::__IDL_TYPE),*]
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

        // --- Client-side instruction struct ---
        let ix_struct_name = syn::Ident::new(
            &to_camel_case(&fn_name_str),
            fn_name.span(),
        );
        let disc_literal_bytes: Vec<_> = disc_bytes.iter().map(|b| quote! { #b }).collect();
        instruction_structs.push(quote! {
            #[derive(anchor_lang_v2::AnchorSerialize, anchor_lang_v2::AnchorDeserialize)]
            pub struct #ix_struct_name {
                #(pub #extra_arg_names: #extra_arg_types,)*
            }
            impl anchor_lang_v2::Discriminator for #ix_struct_name {
                const DISCRIMINATOR: &'static [u8] = &[#(#disc_literal_bytes),*];
            }
            impl anchor_lang_v2::InstructionData for #ix_struct_name {}
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
            #disc_u64 => __handlers::#fn_name(__program_id, __accounts, __ix_data),
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
                anchor_lang_v2::run_handler::<#accounts_type>(__program_id, __accounts, |__ctx| {
                    #mod_name::#fn_name(__ctx, #(#extra_arg_names),*)
                })
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
            if *__program_id != crate::ID {
                return Err(solana_program_error::ProgramError::IncorrectProgramId);
            }
            let (__disc, __ix_data) = anchor_lang_v2::parse_instruction(__data)?;
            (match __disc {
                #(#dispatch_arms)*
                _ => Err(anchor_lang_v2::ErrorCode::InstructionFallbackNotFound.into()),
            }).map_err(|e| e.into())
        }

        mod __handlers {
            use super::*;
            use anchor_lang_v2::TryAccounts as _;
            #(#handler_wrappers)*
        }

        /// Client-side instruction structs for off-chain use.
        #[cfg(feature = "cpi")]
        pub mod instruction {
            extern crate alloc;
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
