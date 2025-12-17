use crate::{AccountField, AccountsStruct};
use quote::quote;

// Generates the `ToAccountInfos` trait implementation.
pub fn generate(accs: &AccountsStruct) -> proc_macro2::TokenStream {
    let name = &accs.ident;

    let (impl_gen, ty_gen, where_clause) = accs.generics.split_for_impl();

    let to_acc_infos: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &AccountField| {
            let name = &f.ident();
            quote! { account_infos.extend(self.#name.to_account_infos()); }
        })
        .collect();
    quote! {
        #[automatically_derived]
        impl #impl_gen anchor_lang::ToAccountInfos for #name #ty_gen #where_clause{
            fn to_account_infos(&self) -> Vec<anchor_lang::AccountInfo> {
                let mut account_infos = vec![];

                #(#to_acc_infos)*

                account_infos
            }
        }
    }
}
