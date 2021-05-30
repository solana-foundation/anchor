use crate::Program;
use heck::SnakeCase;
use quote::quote;

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    let mut accounts = std::collections::HashSet::new();

    // Go through state accounts.
    if let Some(state) = &program.state {
        // Ctor.
        if let Some((_ctor, ctor_accounts)) = &state.ctor_and_anchor {
            let macro_name = format!(
                "__client_accounts_{}",
                ctor_accounts.to_string().to_snake_case()
            );
            accounts.insert(macro_name);
        }
        // Methods.
        if let Some((_impl_block, methods)) = &state.impl_block_and_methods {
            for ix in methods {
                let anchor_ident = &ix.anchor_ident;
                // TODO: move to fn and share with accounts.rs.
                let macro_name = format!(
                    "__client_accounts_{}",
                    anchor_ident.to_string().to_snake_case()
                );
                accounts.insert(macro_name);
            }
        }
    }

    // Go through instruction accounts.
    for ix in &program.ixs {
        let anchor_ident = &ix.anchor_ident;
        // TODO: move to fn and share with accounts.rs.
        let macro_name = format!(
            "__client_accounts_{}",
            anchor_ident.to_string().to_snake_case()
        );
        accounts.insert(macro_name);
    }

    // Build the tokens from all accounts
    let account_structs: Vec<proc_macro2::TokenStream> = accounts
        .iter()
        .map(|macro_name: &String| {
            let macro_name: proc_macro2::TokenStream = macro_name.parse().unwrap();
            quote! {
                pub use crate::#macro_name::*;
            }
        })
        .collect();

    // TODO: calculate the account size and add it as a constant field to
    //       each struct here. This is convenient for Rust clients.

    quote! {
        /// An Anchor generated module, providing a set of structs
        /// mirroring the structs deriving `Accounts`, where each field is
        /// a `Pubkey`. This is useful for specifying accounts for a client.
        pub mod accounts {
            #(#account_structs)*
        }
    }
}
