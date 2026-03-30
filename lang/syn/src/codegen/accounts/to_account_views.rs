use {
    crate::{
        codegen::accounts::{generics, ParsedGenerics},
        AccountField, AccountsStruct,
    },
    quote::quote,
};

// Generates the `ToAccountViews` trait implementation.
pub fn generate(accs: &AccountsStruct) -> proc_macro2::TokenStream {
    let name = &accs.ident;
    let ParsedGenerics {
        combined_generics,
        trait_generics: _,
        struct_generics,
        where_clause,
    } = generics(accs);

    let to_acc_infos: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &AccountField| {
            let name = &f.ident();
            quote! { account_infos.extend(self.#name.to_account_views()); }
        })
        .collect();
    quote! {
        #[automatically_derived]
        impl<#combined_generics> anchor_lang::ToAccountViews for #name <#struct_generics> #where_clause{
            fn to_account_views(&self) -> Vec<anchor_lang::pinocchio_runtime::account_view::AccountView> {
                let mut account_infos = vec![];

                #(#to_acc_infos)*

                account_infos
            }
        }
    }
}
