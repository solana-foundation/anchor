use std::str::FromStr;

use crate::{AccountField, AccountsStruct, Ty};
use heck::SnakeCase;
use quote::quote;

// Generates the private `__cpi_client_accounts` mod implementation, containing
// a generated struct mapping 1-1 to the `Accounts` struct, except with
// `AccountInfo`s as the types. This is generated for CPI clients.
pub fn generate(accs: &AccountsStruct) -> proc_macro2::TokenStream {
    let name = &accs.ident;
    let account_mod_name: proc_macro2::TokenStream = format!(
        "__cpi_client_accounts_{}",
        accs.ident.to_string().to_snake_case()
    )
    .parse()
    .unwrap();

    let account_struct_fields: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &AccountField| match f {
            AccountField::CompositeField(s) => {
                let name = &s.ident;
                let docs = if let Some(ref docs) = s.docs {
                    docs.iter()
                        .map(|docs_line| {
                            proc_macro2::TokenStream::from_str(&format!(
                                "#[doc = r#\"{}\"#]",
                                docs_line
                            ))
                            .unwrap()
                        })
                        .collect()
                } else {
                    quote!()
                };
                let symbol: proc_macro2::TokenStream = format!(
                    "__cpi_client_accounts_{0}::{1}",
                    s.symbol.to_snake_case(),
                    s.symbol,
                )
                .parse()
                .unwrap();
                quote! {
                    #docs
                    pub #name: #symbol<'info>
                }
            }
            AccountField::Field(f) => {
                let name = &f.ident;
                let docs = if let Some(ref docs) = f.docs {
                    docs.iter()
                        .map(|docs_line| {
                            proc_macro2::TokenStream::from_str(&format!(
                                "#[doc = r#\"{}\"#]",
                                docs_line
                            ))
                            .unwrap()
                        })
                        .collect()
                } else {
                    quote!()
                };
                quote! {
                    #docs
                    pub #name: anchor_lang::solana_program::account_info::AccountInfo<'info>
                }
            }
        })
        .collect();

    let account_struct_metas: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &AccountField| match f {
            AccountField::CompositeField(s) => {
                let name = &s.ident;
                quote! {
                    account_metas.extend(self.#name.to_account_metas(None));
                }
            }
            AccountField::Field(f) => {
                let is_signer = match f.ty {
                    Ty::Signer => true,
                    _ => f.constraints.is_signer(),
                };
                let is_signer = match is_signer {
                    false => quote! {false},
                    true => quote! {true},
                };
                let meta = match f.constraints.is_mutable() {
                    false => quote! { anchor_lang::solana_program::instruction::AccountMeta::new_readonly },
                    true => quote! { anchor_lang::solana_program::instruction::AccountMeta::new },
                };
                let name = &f.ident;
                quote! {
                    account_metas.push(#meta(anchor_lang::Key::key(&self.#name), #is_signer));
                }
            }
        })
        .collect();

    let account_struct_infos: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &AccountField| match f {
            AccountField::CompositeField(s) => {
                let name = &s.ident;
                quote! {
                    account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.#name));
                }
            }
            AccountField::Field(f) => {
                let name = &f.ident;
                quote! {
                    account_infos.push(anchor_lang::ToAccountInfo::to_account_info(&self.#name));
                }
            }
        })
        .collect();

    // Re-export all composite account structs (i.e. other structs deriving
    // accounts embedded into this struct. Required because, these embedded
    // structs are *not* visible from the #[program] macro, which is responsible
    // for generating the `accounts` mod, which aggregates all the the generated
    // accounts used for structs.
    let re_exports: Vec<proc_macro2::TokenStream> = {
        // First, dedup the exports.
        let mut re_exports = std::collections::HashSet::new();
        for f in accs.fields.iter().filter_map(|f: &AccountField| match f {
            AccountField::CompositeField(s) => Some(s),
            AccountField::Field(_) => None,
        }) {
            re_exports.insert(format!(
                "__cpi_client_accounts_{0}::{1}",
                f.symbol.to_snake_case(),
                f.symbol,
            ));
        }

        re_exports
            .iter()
            .map(|symbol: &String| {
                let symbol: proc_macro2::TokenStream = symbol.parse().unwrap();
                quote! {
                    pub use #symbol;
                }
            })
            .collect()
    };
    let generics = if account_struct_fields.is_empty() {
        quote! {}
    } else {
        quote! {<'info>}
    };
    let struct_doc = proc_macro2::TokenStream::from_str(&format!(
        "#[doc = \" Generated CPI struct of the accounts for [`{}`].\"]",
        name
    ))
    .unwrap();
    quote! {
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// [`cpi::accounts`] module (also generated), which re-exports this.
        pub(crate) mod #account_mod_name {
            use super::*;

            #(#re_exports)*

            #struct_doc
            pub struct #name #generics {
                #(#account_struct_fields),*
            }

            #[automatically_derived]
            impl #generics anchor_lang::ToAccountMetas for #name #generics {
                fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = vec![];
                    #(#account_struct_metas)*
                    account_metas
                }
            }

            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for #name #generics {
                fn to_account_infos(&self) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                    let mut account_infos = vec![];
                    #(#account_struct_infos)*
                    account_infos
                }
            }
        }
    }
}
