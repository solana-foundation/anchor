use crate::codegen::accounts::generics;
use crate::{AccountField, AccountsStruct};
use quote::quote;

// Generates the `ToAccountMetas` trait implementation.
pub fn generate(accs: &AccountsStruct) -> proc_macro2::TokenStream {
    let name = &accs.ident;
    let (combined_generics, _trait_generics, strct_generics) = generics(accs);

    let to_acc_metas: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &AccountField| {
            let (name, is_signer) = match f {
                AccountField::CompositeField(s) => (&s.ident, quote! {None}),
                AccountField::Field(f) => {
                    let is_signer = match f.constraints.is_signer() {
                        false => quote! {None},
                        true => quote! {Some(true)},
                    };
                    (&f.ident, is_signer)
                }
            };
            quote! {
                account_metas.extend(self.#name.to_account_metas(#is_signer));
            }
        })
        .collect();
    quote! {
        impl#combined_generics anchor_lang::ToAccountMetas for #name#strct_generics {
            fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];

                #(#to_acc_metas)*

                account_metas
            }
        }
    }
}
