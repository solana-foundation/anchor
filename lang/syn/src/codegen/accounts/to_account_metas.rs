use crate::codegen::accounts::{generics, ParsedGenerics};
use crate::{AccountField, AccountsStruct};
use quote::quote;

// Generates the `ToAccountMetas` trait implementation.
pub fn generate(accs: &AccountsStruct) -> proc_macro2::TokenStream {
    let name = &accs.ident;
    let ParsedGenerics {
        combined_generics,
        trait_generics,
        struct_generics,
        where_clause,
    } = generics(accs);

    let to_acc_metas: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &AccountField| {
            let (name, is_signer, is_optional) = match f {
                AccountField::CompositeField(s) => (&s.ident, quote! {None}, false),
                AccountField::Field(f) => {
                    let is_signer = match f.constraints.is_signer() {
                        false => quote! {None},
                        true => quote! {Some(true)},
                    };
                    (&f.ident, is_signer, f.is_optional)
                }
            };
            if is_optional {
                quote! {
                    if let Some(#name) = &self.#name {
                        account_metas.extend(#name.to_account_metas(#is_signer));
                    } else {
                        account_metas.push(anchor_lang::pinocchio_runtime::instruction::AccountMeta::readonly(crate::ID));
                    }
                }
            } else {
                quote! {
                    account_metas.extend(self.#name.to_account_metas(#is_signer));
                }
            }
        })
        .collect();

    // Extract the lifetime from trait_generics (should be a single GenericParam::Lifetime)
    let trait_lifetime = match trait_generics.iter().next() {
        Some(syn::GenericParam::Lifetime(lifetime_def)) => &lifetime_def.lifetime,
        _ => panic!("trait_generics should contain a single lifetime parameter"),
    };

    quote! {
        #[automatically_derived]
        impl<#combined_generics> anchor_lang::ToAccountMetas<#trait_lifetime> for #name <#struct_generics> #where_clause{
            fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_lang::pinocchio_runtime::instruction::AccountMeta<'_>> {
                let mut account_metas = vec![];

                #(#to_acc_metas)*

                account_metas
            }
        }
    }
}
