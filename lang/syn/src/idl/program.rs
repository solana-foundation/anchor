use std::path::PathBuf;

use anyhow::{anyhow, Result};
use heck::CamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::{
    common::{gen_print_section, get_idl_module_path, get_no_docs},
    defined::gen_idl_type,
};
use crate::{
    parser::{context::CrateContext, docs},
    Program,
};

/// Generate the IDL build print function for the program module.
pub fn gen_idl_print_fn_program(program: &Program) -> TokenStream {
    check_safety_comments().unwrap_or_else(|e| panic!("Safety checks failed: {e}"));

    let idl = get_idl_module_path();
    let no_docs = get_no_docs();

    let name = program.name.to_string();
    let docs = match &program.docs {
        Some(docs) if !no_docs => quote! { vec![#(#docs.into()),*] },
        _ => quote! { vec![] },
    };

    let (instructions, defined) = program
        .ixs
        .iter()
        .flat_map(|ix| -> Result<_> {
            let name = ix.ident.to_string();
            let name_pascal = format_ident!("{}", name.to_camel_case());
            let ctx_ident = &ix.anchor_ident;
            let discriminator = quote! {
                <crate::instruction::#name_pascal as anchor_lang::Discriminator>::DISCRIMINATOR
            };

            let docs = match &ix.docs {
                Some(docs) if !no_docs => quote! { vec![#(#docs.into()),*] },
                _ => quote! { vec![] },
            };

            let (args, mut defined) = ix
                .args
                .iter()
                .map(|arg| {
                    let name = arg.name.to_string();
                    let docs = match docs::parse(&arg.raw_arg.attrs) {
                        Some(docs) if !no_docs => quote! { vec![#(#docs.into()),*] },
                        _ => quote! { vec![] },
                    };
                    let (ty, defined) = gen_idl_type(&arg.raw_arg.ty, &[])?;

                    Ok((
                        quote! {
                            #idl::IdlField {
                                name: #name.into(),
                                docs: #docs,
                                ty: #ty,
                            }
                        },
                        defined,
                    ))
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .unzip::<_, Vec<_>, Vec<_>, Vec<_>>();

            let returns = match gen_idl_type(&ix.returns.ty, &[]) {
                Ok((ty, def)) => {
                    defined.push(def);
                    quote! { Some(#ty) }
                }
                _ => quote! { None },
            };

            Ok((
                quote! {
                    #idl::IdlInstruction {
                        name: #name.into(),
                        docs: #docs,
                        discriminator: #discriminator.into(),
                        accounts: #ctx_ident::__anchor_private_gen_idl_accounts(
                            &mut accounts,
                            &mut types,
                        ),
                        args: vec![#(#args),*],
                        returns: #returns,
                    }
                },
                defined,
            ))
        })
        .unzip::<_, Vec<_>, Vec<_>, Vec<_>>();
    let defined = defined.into_iter().flatten().flatten().collect::<Vec<_>>();

    let fn_body = gen_print_section(
        "program",
        quote! {
            let mut accounts: std::collections::BTreeMap<String, #idl::IdlAccount> =
                std::collections::BTreeMap::new();
            let mut types: std::collections::BTreeMap<String, #idl::IdlTypeDef> =
                std::collections::BTreeMap::new();

            #(
                if let Some(ty) = <#defined>::create_type() {
                    types.insert(<#defined>::get_full_path(), ty);
                    <#defined>::insert_types(&mut types);
                }
            );*

            #idl::Idl {
                address: Default::default(),
                metadata: #idl::IdlMetadata {
                    name: #name.into(),
                    version: env!("CARGO_PKG_VERSION").into(),
                    spec: #idl::IDL_SPEC.into(),
                    description: option_env!("CARGO_PKG_DESCRIPTION")
                        .filter(|d| !d.is_empty())
                        .map(|d| d.into()),
                    repository: option_env!("CARGO_PKG_REPOSITORY")
                        .filter(|r| !r.is_empty())
                        .map(|r| r.into()),
                    dependencies: Default::default(),
                    contact: Default::default(),
                    deployments: Default::default(),
                },
                docs: #docs,
                instructions: vec![#(#instructions),*],
                accounts: accounts.into_values().collect(),
                events: Default::default(),
                errors: Default::default(),
                types: types.into_values().collect(),
                constants: Default::default(),
            }
        },
    );

    quote! {
        #[test]
        pub fn __anchor_private_print_idl_program() {
            #fn_body
        }
    }
}

/// Check safety comments.
fn check_safety_comments() -> Result<()> {
    let skip_lint = option_env!("ANCHOR_IDL_BUILD_SKIP_LINT")
        .map(|val| val == "TRUE")
        .unwrap_or_default();
    if skip_lint {
        return Ok(());
    }

    std::env::var("ANCHOR_IDL_BUILD_PROGRAM_PATH")
        .map(PathBuf::from)
        .map(|path| path.join("src").join("lib.rs"))
        .map_err(|_| anyhow!("Failed to get program path"))
        .map(CrateContext::parse)?
        .map_err(|e| anyhow!("Failed to parse crate: {e}"))?
        .safety_checks()
}
