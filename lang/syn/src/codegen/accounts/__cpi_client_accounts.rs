use std::str::FromStr;

use crate::{AccountField, AccountsStruct, Ty};
use heck::SnakeCase;
use quote::quote;

// Generates the private `__cpi_client_accounts` mod implementation, containing
// a generated struct mapping 1-1 to the `Accounts` struct, except with
// `AccountInfo`s as the types. This is generated for CPI clients.
pub fn generate(
    accs: &AccountsStruct,
    program_id: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
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
                                "#[doc = r#\"{docs_line}\"#]"
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
                                "#[doc = r#\"{docs_line}\"#]"
                            ))
                            .unwrap()
                        })
                        .collect()
                } else {
                    quote!()
                };
                if f.is_optional {
                    quote! {
                        #docs
                        pub #name: Option<anchor_lang::pinocchio_runtime::account_info::AccountInfo>
                    }
                } else {
                    quote! {
                        #docs
                        pub #name: anchor_lang::pinocchio_runtime::account_info::AccountInfo
                    }
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
                let name = &f.ident;
                let is_mutable = f.constraints.is_mutable();
                if f.is_optional {
                    quote! {
                        if let Some(#name) = &self.#name {
                            // For AccountInfo, use address() which returns &Pubkey directly
                            let account_ref = #name;
                            let meta = match (#is_mutable, #is_signer) {
                                (false, false) => anchor_lang::pinocchio_runtime::instruction::AccountMeta::readonly(account_ref.address()),
                                (false, true) => anchor_lang::pinocchio_runtime::instruction::AccountMeta::readonly_signer(account_ref.address()),
                                (true, false) => anchor_lang::pinocchio_runtime::instruction::AccountMeta::writable(account_ref.address()),
                                (true, true) => anchor_lang::pinocchio_runtime::instruction::AccountMeta::writable_signer(account_ref.address()),
                            };
                            account_metas.push(meta);
                        } else {
                            account_metas.push(anchor_lang::pinocchio_runtime::instruction::AccountMeta::readonly(#program_id));
                        }
                    }
                } else {
                    quote! {
                        // For AccountInfo, use address() which returns &Pubkey directly
                        let account_ref = &self.#name;
                        let meta = match (#is_mutable, #is_signer) {
                            (false, false) => anchor_lang::pinocchio_runtime::instruction::AccountMeta::readonly(account_ref.address()),
                            (false, true) => anchor_lang::pinocchio_runtime::instruction::AccountMeta::readonly_signer(account_ref.address()),
                            (true, false) => anchor_lang::pinocchio_runtime::instruction::AccountMeta::writable(account_ref.address()),
                            (true, true) => anchor_lang::pinocchio_runtime::instruction::AccountMeta::writable_signer(account_ref.address()),
                        };
                        account_metas.push(meta);
                    }
                }
            }
        })
        .collect();

    let account_struct_infos: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &AccountField| {
            let name = &f.ident();
            quote! {
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.#name));
            }
        })
        .collect();

    // Re-export all composite account structs (i.e. other structs deriving
    // accounts embedded into this struct. Required because, these embedded
    // structs are *not* visible from the #[program] macro, which is responsible
    // for generating the `accounts` mod, which aggregates all the generated
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
    // Check if there are any composite fields (which need lifetimes)
    // Regular fields are always AccountInfo in this module, so they don't need lifetimes
    let has_composite_fields = accs
        .fields
        .iter()
        .any(|f| matches!(f, AccountField::CompositeField(_)));

    let generics = if account_struct_fields.is_empty() || !has_composite_fields {
        quote! {}
    } else {
        quote! {<'info>}
    };

    let struct_doc = proc_macro2::TokenStream::from_str(&format!(
        "#[doc = \" Generated CPI struct of the accounts for [`{name}`].\"]"
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
            impl<'info> #generics anchor_lang::ToAccountMetas<'info> for #name #generics {
                fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_lang::pinocchio_runtime::instruction::AccountMeta> {
                    let mut account_metas = vec![];
                    #(#account_struct_metas)*
                    account_metas
                }
            }

            #[automatically_derived]
            impl anchor_lang::ToAccountInfos for #name #generics {
                fn to_account_infos(&self) -> Vec<anchor_lang::pinocchio_runtime::account_info::AccountInfo> {
                    let mut account_infos = vec![];
                    #(#account_struct_infos)*
                    account_infos
                }
            }
        }
    }
}
