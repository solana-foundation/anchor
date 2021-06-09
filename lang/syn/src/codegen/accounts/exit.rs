use crate::codegen::accounts::generics;
use crate::{AccountField, AccountsStruct};
use quote::quote;

// Generates the `Exit` trait implementation.
pub fn generate(accs: &AccountsStruct) -> proc_macro2::TokenStream {
    let name = &accs.ident;
    let (combined_generics, trait_generics, strct_generics) = generics(accs);

    let on_save: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|af: &AccountField| match af {
            AccountField::CompositeField(s) => {
                let name = &s.ident;
                quote! {
                    anchor_lang::AccountsExit::exit(&self.#name, program_id)?;
                }
            }
            AccountField::Field(f) => {
                let ident = &f.ident;
                if f.constraints.is_close() {
                    let close_target = &f.constraints.close.as_ref().unwrap().sol_dest;
                    quote! {
                        anchor_lang::AccountsClose::close(
                            &self.#ident,
                            self.#close_target.to_account_info(),
                        )?;
                    }
                } else {
                    match f.constraints.is_mutable() {
                        false => quote! {},
                        true => quote! {
                            anchor_lang::AccountsExit::exit(&self.#ident, program_id)?;
                        },
                    }
                }
            }
        })
        .collect();
    quote! {
        impl#combined_generics anchor_lang::AccountsExit#trait_generics for #name#strct_generics {
            fn exit(&self, program_id: &anchor_lang::solana_program::pubkey::Pubkey) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                #(#on_save)*
                Ok(())
            }
        }
    }
}
