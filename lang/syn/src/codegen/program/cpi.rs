use {
    crate::{
        codegen::program::common::{generate_ix_variant, generate_ix_variant_name},
        Program,
    },
    heck::SnakeCase,
    quote::{quote, ToTokens},
    syn::{self, Type},
};

/// Matches [`crate::parser::accounts::is_field_primitive`] — composite (non-primitive) fields
/// force `<'info>` on generated CPI client account structs.
fn cpi_client_accounts_has_lifetime(program: &Program, accounts_ident: &syn::Ident) -> bool {
    let Some((_, items)) = &program.program_mod.content else {
        return false;
    };
    let Some(st) = items.iter().find_map(|item| match item {
        syn::Item::Struct(s) if &s.ident == accounts_ident => Some(s),
        _ => None,
    }) else {
        return false;
    };

    fn peel(ty: &Type) -> &Type {
        match ty {
            Type::Reference(r) => peel(&r.elem),
            _ => ty,
        }
    }

    fn type_needs_cpi_lifetime(ty: &Type) -> bool {
        match peel(ty) {
            Type::Path(p) => {
                let Some(first) = p.path.segments.first() else {
                    return false;
                };
                let name = first.ident.to_string();
                match name.as_str() {
                    "Option" | "Box" => {
                        if let syn::PathArguments::AngleBracketed(args) = &first.arguments {
                            if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                                return type_needs_cpi_lifetime(inner);
                            }
                        }
                        false
                    }
                    "Sysvar" | "AccountInfo" | "UncheckedAccount" | "AccountLoader" | "Account"
                    | "LazyAccount" | "Migration" | "Program" | "Interface"
                    | "InterfaceAccount" | "Signer" | "SystemAccount" | "ProgramData" => false,
                    _ => true,
                }
            }
            _ => false,
        }
    }

    st.fields.iter().any(|f| type_needs_cpi_lifetime(&f.ty))
}

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    // Generate cpi methods for global methods.
    let global_cpi_methods: Vec<proc_macro2::TokenStream> = program
        .ixs
        .iter()
        .map(|ix| {
            let accounts_ident: proc_macro2::TokenStream = format!("crate::cpi::accounts::{}", &ix.anchor_ident.to_string()).parse().unwrap();
            let cpi_method = {
                let name = &ix.raw_method.sig.ident;
                let name_str = name.to_string();
                let ix_variant = match generate_ix_variant(&name_str, &ix.args) {
                    Ok(v) => v,
                    Err(e) => {
                        let err = e.to_string();
                        return quote! { compile_error!(concat!("error generating ix variant: `", #err, "`")) };
                    }
                };
                let method_name = &ix.ident;
                let args: Vec<&syn::PatType> = ix.args.iter().map(|arg| &arg.raw_arg).collect();
                let discriminator = match generate_ix_variant_name(&name_str) {
                    Ok(name) => quote! { <instruction::#name as anchor_lang::Discriminator>::DISCRIMINATOR },
                    Err(e) => {
                        let err = e.to_string();
                        return quote! { compile_error!(concat!("error generating ix variant name: `", #err, "`")) };
                    }
                };
                let ret_type = &ix.returns.ty.to_token_stream();
                let ix_cfgs = &ix.cfgs;
                let (method_ret, maybe_return) = match ret_type.to_string().as_str() {
                    "()" => (quote! {anchor_lang::Result<()> }, quote! { Ok(()) }),
                    _ => (
                        quote! { anchor_lang::Result<crate::cpi::Return::<#ret_type>> },
                        quote! { Ok(crate::cpi::Return::<#ret_type> { phantom: crate::cpi::PhantomData }) }
                    )
                };

                let cpi_lt = cpi_client_accounts_has_lifetime(program, &ix.anchor_ident);
                let method_generics = if cpi_lt {
                    quote! { <'info> }
                } else {
                    quote! {}
                };
                let accounts_ty_generics = if cpi_lt {
                    quote! { <'info> }
                } else {
                    quote! {}
                };

                quote! {
                    #(#ix_cfgs)*
                    pub fn #method_name #method_generics(
                        ctx: anchor_lang::context::CpiContext<'_, '_, #accounts_ident #accounts_ty_generics>,
                        #(#args),*
                    ) -> #method_ret {
                        let ix_data = {
                            let ix = instruction::#ix_variant;
                            let mut data = Vec::with_capacity(256);
                            data.extend_from_slice(#discriminator);
                            AnchorSerialize::serialize(&ix, &mut data)
                                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
                            data
                        };
                        let accounts = ctx.to_account_metas(None);
                        let ix = anchor_lang::pinocchio_runtime::instruction::InstructionView {
                            program_id: &ctx.program_id,
                            accounts: accounts.as_slice(),
                            data: ix_data.as_slice(),
                        };
                        let acc_infos = ctx.to_account_infos();
                        anchor_lang::pinocchio_runtime::program::invoke_signed_with_slice(
                            &ix,
                            acc_infos.as_slice(),
                            ctx.signer_seeds,
                        )
                        .map_err(anchor_lang::error::Error::from)?;
                        #maybe_return
                    }
                }
            };

            cpi_method
        })
        .collect();

    let accounts = generate_accounts(program);

    quote! {
        #[cfg(feature = "cpi")]
        pub mod cpi {
            use super::*;
            use std::marker::PhantomData;


            pub struct Return<T> {
                phantom: std::marker::PhantomData<T>
            }

            impl<T: AnchorDeserialize> Return<T> {
                pub fn get(&self) -> T {
                    let rd = anchor_lang::pinocchio_runtime::program::get_return_data().unwrap();
                    T::try_from_slice(rd.as_slice()).unwrap()
                }
            }

            #(#global_cpi_methods)*

            #accounts
        }
    }
}

pub fn generate_accounts(program: &Program) -> proc_macro2::TokenStream {
    let mut accounts = std::collections::HashMap::new();

    // Go through instruction accounts.
    for ix in &program.ixs {
        let anchor_ident = &ix.anchor_ident;
        // TODO: move to fn and share with accounts.rs.
        let macro_name = format!(
            "__cpi_client_accounts_{}",
            anchor_ident.to_string().to_snake_case()
        );
        let cfgs = &ix.cfgs;
        accounts.insert(macro_name, cfgs.as_slice());
    }

    // Build the tokens from all accounts
    let account_structs: Vec<proc_macro2::TokenStream> = accounts
        .iter()
        .map(|(macro_name, cfgs)| {
            let macro_name: proc_macro2::TokenStream = macro_name.parse().unwrap();
            quote! {
                #(#cfgs)*
                pub use crate::#macro_name::*;
            }
        })
        .collect();

    quote! {
        /// An Anchor generated module, providing a set of structs
        /// mirroring the structs deriving `Accounts`, where each field is
        /// an `AccountInfo`. This is useful for CPI.
        pub mod accounts {
            #(#account_structs)*
        }
    }
}
