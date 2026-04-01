use {
    crate::{AccountField, AccountsStruct, Ty},
    heck::SnakeCase,
    quote::{format_ident, quote, ToTokens},
    std::str::FromStr,
};

// Generates the private `__cpi_client_accounts` mod implementation, containing
// a generated struct mapping 1-1 to the `Accounts` struct, except with
// `AccountView`s as the types. This is generated for CPI clients.
pub fn generate(
    accs: &AccountsStruct,
    program_id: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let name = &accs.ident;
    let name_cpi = format_ident!("{}Cpi", name);
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
                let cpi_symbol: proc_macro2::TokenStream = format!(
                    "__cpi_client_accounts_{}::{}Cpi",
                    s.symbol.to_snake_case(),
                    s.symbol
                )
                .parse()
                .unwrap();
                quote! {
                    #docs
                    pub #name: #cpi_symbol<'info, 'cpi>
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
                let field_ty = match &f.ty {
                    Ty::Account(account_ty) => {
                        let ty = &account_ty.account_type_path;
                        if f.constraints.is_mutable() {
                            quote! { anchor_lang::context::CpiAccountMut<'cpi, #ty> }
                        } else {
                            quote! { anchor_lang::context::CpiAccountRef<'cpi, #ty> }
                        }
                    }
                    _ => quote! { anchor_lang::pinocchio_runtime::account_view::AccountView },
                };

                if f.is_optional {
                    quote! {
                        #docs
                        pub #name: Option<#field_ty>
                    }
                } else {
                    quote! {
                        #docs
                        pub #name: #field_ty
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
                account_infos.extend(anchor_lang::ToAccountViews::to_account_views(&self.#name));
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
    let needs_cpi_lifetime = accs.fields.iter().any(|af| match af {
        AccountField::CompositeField(_) => true,
        AccountField::Field(f) => matches!(f.ty, Ty::Account(_)),
    });
    let needs_info_lifetime = accs
        .fields
        .iter()
        .any(|af| matches!(af, AccountField::CompositeField(_)));

    let mut extra_decl_params: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut extra_arg_params: Vec<proc_macro2::TokenStream> = Vec::new();
    for p in accs.generics.params.iter() {
        match p {
            syn::GenericParam::Lifetime(_) => {}
            syn::GenericParam::Type(ty) => {
                extra_decl_params.push(ty.ident.to_token_stream());
                extra_arg_params.push(ty.ident.to_token_stream());
            }
            syn::GenericParam::Const(c) => {
                extra_decl_params.push(c.to_token_stream());
                extra_arg_params.push(c.ident.to_token_stream());
            }
        }
    }

    let extra_decl_params_comma = if extra_decl_params.is_empty() {
        quote!()
    } else {
        quote! {, #(#extra_decl_params),*}
    };
    let extra_arg_params_comma = if extra_arg_params.is_empty() {
        quote!()
    } else {
        quote! {, #(#extra_arg_params),*}
    };

    let has_extra_generics = !extra_decl_params.is_empty();

    let generics_decl = if account_struct_fields.is_empty() {
        quote! {}
    } else if needs_cpi_lifetime {
        if needs_info_lifetime {
            quote! {<'info, 'cpi #extra_decl_params_comma>}
        } else {
            quote! {<'cpi #extra_decl_params_comma>}
        }
    } else if has_extra_generics {
        quote! {<#(#extra_decl_params),*>}
    } else {
        quote! {}
    };

    let generics_args = if account_struct_fields.is_empty() {
        quote! {}
    } else if needs_cpi_lifetime {
        if needs_info_lifetime {
            quote! {<'info, 'cpi #extra_arg_params_comma>}
        } else {
            quote! {<'cpi #extra_arg_params_comma>}
        }
    } else if has_extra_generics {
        quote! {<#(#extra_arg_params),*>}
    } else {
        quote! {}
    };

    let to_account_views_impl_generics = if account_struct_fields.is_empty() {
        quote! {}
    } else {
        generics_decl.clone()
    };
    let where_clause_tokens = accs.generics.where_clause.as_ref().map(|wc| quote! { #wc });
    let struct_doc = proc_macro2::TokenStream::from_str(&format!(
        "#[doc = \" Generated CPI struct of the accounts for [`{name}`].\"]"
    ))
    .unwrap();
    quote! {
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountView.
        ///
        /// To access the struct in this module, one should use the sibling
        /// [`cpi::accounts`] module (also generated), which re-exports this.
        pub(crate) mod #account_mod_name {
            use super::*;

            #(#re_exports)*

            #struct_doc
            pub struct #name #generics_decl #where_clause_tokens {
                #(#account_struct_fields),*
            }

            #[allow(unused_lifetimes)]
            pub type #name_cpi<'info, 'cpi #extra_decl_params_comma> = #name #generics_args;

            #[automatically_derived]
            impl #generics_decl anchor_lang::ToAccountMetas for #name #generics_args #where_clause_tokens {
                fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_lang::pinocchio_runtime::instruction::AccountMeta> {
                    let mut account_metas = vec![];
                    #(#account_struct_metas)*
                    account_metas
                }
            }

            #[automatically_derived]
            impl #to_account_views_impl_generics anchor_lang::ToAccountViews for #name #generics_args #where_clause_tokens {
                fn to_account_views(&self) -> Vec<anchor_lang::pinocchio_runtime::account_view::AccountView> {
                    let mut account_infos = vec![];
                    #(#account_struct_infos)*
                    account_infos
                }
            }
        }
    }
}
