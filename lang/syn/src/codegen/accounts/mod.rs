use crate::AccountsStruct;
use quote::quote;

mod __client_accounts;
mod constraints;
mod exit;
mod to_account_infos;
mod to_account_metas;
mod try_accounts;

pub fn generate(accs: &AccountsStruct) -> proc_macro2::TokenStream {
    let impl_try_accounts = try_accounts::generate(accs);
    let impl_to_account_infos = to_account_infos::generate(accs);
    let impl_to_account_metas = to_account_metas::generate(accs);
    let impl_exit = exit::generate(accs);

    let __client_accounts_mod = __client_accounts::generate(accs);

    quote! {
        #impl_try_accounts
        #impl_to_account_infos
        #impl_to_account_metas
        #impl_exit

        #__client_accounts_mod
    }
}

fn generics(
    accs: &AccountsStruct,
) -> (
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
) {
    match accs.generics.lt_token {
        None => (quote! {<'info>}, quote! {<'info>}, quote! {}),
        Some(_) => {
            let g = &accs.generics;
            (quote! {#g}, quote! {#g}, quote! {#g})
        }
    }
}
