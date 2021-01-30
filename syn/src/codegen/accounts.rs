use crate::{
    AccountField, AccountsStruct, CompositeField, Constraint, ConstraintBelongsTo,
    ConstraintLiteral, ConstraintOwner, ConstraintRentExempt, ConstraintSeeds, ConstraintSigner,
    Field, Ty,
};
use heck::SnakeCase;
use quote::quote;

pub fn generate(accs: AccountsStruct) -> proc_macro2::TokenStream {
    // Deserialization for each field.
    let deser_fields: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|af: &AccountField| match af {
            AccountField::AccountsStruct(s) => {
                let name = &s.ident;
                let ty = &s.raw_field.ty;
                quote! {
                    let #name: #ty = anchor_lang::Accounts::try_accounts(program_id, accounts)?;
                }
            }
            AccountField::Field(f) => {
                let name = f.typed_ident();
                match f.is_init {
                    false => quote! {
                        let #name = anchor_lang::Accounts::try_accounts(program_id, accounts)?;
                    },
                    true => quote! {
                        let #name = anchor_lang::AccountsInit::try_accounts_init(program_id, accounts)?;
                    },
                }
            }
        })
        .collect();

    // Constraint checks for each account fields.
    let access_checks: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|af: &AccountField| {
            let checks: Vec<proc_macro2::TokenStream> = match af {
                AccountField::Field(f) => f
                    .constraints
                    .iter()
                    .map(|c| generate_field_constraint(&f, c))
                    .collect(),
                AccountField::AccountsStruct(s) => s
                    .constraints
                    .iter()
                    .map(|c| generate_composite_constraint(&s, c))
                    .collect(),
            };
            quote! {
                #(#checks)*
            }
        })
        .collect();

    // Each field in the final deserialized accounts struct.
    let return_tys: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &AccountField| {
            let name = match f {
                AccountField::AccountsStruct(s) => &s.ident,
                AccountField::Field(f) => &f.ident,
            };
            quote! {
                #name
            }
        })
        .collect();

    // Exit program code-blocks for each account.
    let on_save: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|af: &AccountField| match af {
            AccountField::AccountsStruct(s) => {
                let name = &s.ident;
                quote! {
                    anchor_lang::AccountsExit::exit(&self.#name, program_id)?;
                }
            }
            AccountField::Field(f) => {
                let ident = &f.ident;
                match f.is_mut {
                    false => quote! {},
                    true => quote! {
                        anchor_lang::AccountsExit::exit(&self.#ident, program_id)?;
                    },
                }
            }
        })
        .collect();

    // Implementation for `ToAccountInfos` trait.
    let to_acc_infos: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &AccountField| {
            let name = match f {
                AccountField::AccountsStruct(s) => &s.ident,
                AccountField::Field(f) => &f.ident,
            };
            quote! {
                account_infos.extend(self.#name.to_account_infos());
            }
        })
        .collect();

    // Implementation for `ToAccountMetas` trait.
    let to_acc_metas: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &AccountField| {
            let (name, is_signer) = match f {
                AccountField::AccountsStruct(s) => (&s.ident, quote! {None}),
                AccountField::Field(f) => {
                    let is_signer = match f.is_signer {
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

    let name = &accs.ident;
    let (combined_generics, trait_generics, strct_generics) = match accs.generics.lt_token {
        None => (quote! {<'info>}, quote! {<'info>}, quote! {}),
        Some(_) => {
            let g = &accs.generics;
            (quote! {#g}, quote! {#g}, quote! {#g})
        }
    };

    let account_mod_name: proc_macro2::TokenStream = format!(
        "__client_accounts_{}",
        accs.ident.to_string().to_snake_case()
    )
    .parse()
    .unwrap();

    let account_struct_fields: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &AccountField| match f {
            AccountField::AccountsStruct(s) => {
                let name = &s.ident;
                let symbol: proc_macro2::TokenStream = format!(
                    "__client_accounts_{0}::{1}",
                    s.symbol.to_snake_case(),
                    s.symbol,
                )
                .parse()
                .unwrap();
                quote! {
                    pub #name: #symbol
                }
            }
            AccountField::Field(f) => {
                let name = &f.ident;
                quote! {
                    pub #name: anchor_lang::solana_program::pubkey::Pubkey
                }
            }
        })
        .collect();

    let account_struct_metas: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &AccountField| match f {
            AccountField::AccountsStruct(s) => {
                let name = &s.ident;
                quote! {
                    account_metas.extend(self.#name.to_account_metas(None));
                }
            }
            AccountField::Field(f) => {
                let is_signer = match f.is_signer {
                    false => quote! {false},
                    true => quote! {true},
                };
                let meta = match f.is_mut {
                    false => quote! { anchor_lang::solana_program::instruction::AccountMeta::new_readonly },
                    true => quote! { anchor_lang::solana_program::instruction::AccountMeta::new },
                };
                let name = &f.ident;
                quote! {
                    account_metas.push(#meta(self.#name, #is_signer));
                }
            }
        })
        .collect();

    // Re-export all composite account structs (i.e. other structs deriving
    // accounts embedded into this struct. Required because, these embedded
    // structs are *not* visible from the #[program] macro, which is responsible
    // for generating the `accounts` mod, which aggregates all the the generated
    // accounts used for structs.
    let re_exports: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .filter_map(|f: &AccountField| match f {
            AccountField::AccountsStruct(s) => Some(s),
            AccountField::Field(_) => None,
        })
        .map(|f: &CompositeField| {
            let symbol: proc_macro2::TokenStream = format!(
                "__client_accounts_{0}::{1}",
                f.symbol.to_snake_case(),
                f.symbol,
            )
            .parse()
            .unwrap();
            quote! {
                pub use #symbol;
            }
        })
        .collect();

    quote! {

        mod #account_mod_name {
            use super::*;
            use anchor_lang::prelude::borsh;
            #(#re_exports)*

            #[derive(anchor_lang::AnchorSerialize)]
            pub struct #name {
                #(#account_struct_fields),*
            }

            impl anchor_lang::ToAccountMetas for #name {
                fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = vec![];

                    #(#account_struct_metas)*

                    account_metas
                }
            }
        }

        impl#combined_generics anchor_lang::Accounts#trait_generics for #name#strct_generics {
            #[inline(never)]
            fn try_accounts(program_id: &anchor_lang::solana_program::pubkey::Pubkey, accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>]) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
                // Deserialize each account.
                #(#deser_fields)*

                // Perform constraint checks on each account.
                #(#access_checks)*

                // Success. Return the validated accounts.
                Ok(#name {
                    #(#return_tys),*
                })
            }
        }

        impl#combined_generics anchor_lang::ToAccountInfos#trait_generics for #name#strct_generics {
            fn to_account_infos(&self) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];

                #(#to_acc_infos)*

                account_infos
            }
        }

        impl#combined_generics anchor_lang::ToAccountMetas for #name#strct_generics {
            fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];

                #(#to_acc_metas)*


                account_metas
            }
        }

        impl#combined_generics anchor_lang::AccountsExit#trait_generics for #name#strct_generics {
            fn exit(&self, program_id: &anchor_lang::solana_program::pubkey::Pubkey) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                #(#on_save)*
                Ok(())
            }
        }
    }
}

pub fn generate_field_constraint(f: &Field, c: &Constraint) -> proc_macro2::TokenStream {
    match c {
        Constraint::BelongsTo(c) => generate_constraint_belongs_to(f, c),
        Constraint::Signer(c) => generate_constraint_signer(f, c),
        Constraint::Literal(c) => generate_constraint_literal(c),
        Constraint::Owner(c) => generate_constraint_owner(f, c),
        Constraint::RentExempt(c) => generate_constraint_rent_exempt(f, c),
        Constraint::Seeds(c) => generate_constraint_seeds(f, c),
    }
}

pub fn generate_composite_constraint(
    _f: &CompositeField,
    c: &Constraint,
) -> proc_macro2::TokenStream {
    match c {
        Constraint::Literal(c) => generate_constraint_literal(c),
        _ => panic!("Composite fields can only use literal constraints"),
    }
}

pub fn generate_constraint_belongs_to(
    f: &Field,
    c: &ConstraintBelongsTo,
) -> proc_macro2::TokenStream {
    // todo: assert the field type.

    let target = c.join_target.clone();
    let ident = &f.ident;
    // todo: would be nice if target could be an account info object.
    quote! {
        if &#ident.#target != #target.to_account_info().key {
            return Err(anchor_lang::solana_program::program_error::ProgramError::Custom(1)); // todo: error codes
        }
    }
}

pub fn generate_constraint_signer(f: &Field, _c: &ConstraintSigner) -> proc_macro2::TokenStream {
    let ident = &f.ident;
    let info = match f.ty {
        Ty::AccountInfo => quote! { #ident },
        Ty::ProgramAccount(_) => quote! { #ident.to_account_info() },
        _ => panic!("Invalid syntax: signer cannot be specified."),
    };
    quote! {
        // Don't enforce on CPI, since usually a program is signing and so
        // the `try_accounts` deserializatoin will fail *if* the one
        // tries to manually invoke it.
        //
        // This check will be performed on the other end of the invocation.
        if cfg!(not(feature = "cpi")) {
            if !#info.is_signer {
                return Err(anchor_lang::solana_program::program_error::ProgramError::MissingRequiredSignature);
            }
        }
    }
}

pub fn generate_constraint_literal(c: &ConstraintLiteral) -> proc_macro2::TokenStream {
    let tokens = &c.tokens;
    quote! {
        if !(#tokens) {
            return Err(anchor_lang::solana_program::program_error::ProgramError::Custom(1)); // todo: error codes
        }
    }
}

pub fn generate_constraint_owner(f: &Field, c: &ConstraintOwner) -> proc_macro2::TokenStream {
    let ident = &f.ident;
    let info = match f.ty {
        Ty::AccountInfo => quote! { #ident },
        Ty::ProgramAccount(_) => quote! { #ident.to_account_info() },
        _ => panic!("Invalid syntax: owner cannot be specified."),
    };
    match c {
        ConstraintOwner::Skip => quote! {},
        ConstraintOwner::Program => quote! {
            if #info.owner != program_id {
                return Err(anchor_lang::solana_program::program_error::ProgramError::Custom(1)); // todo: error codes
            }
        },
    }
}

pub fn generate_constraint_rent_exempt(
    f: &Field,
    c: &ConstraintRentExempt,
) -> proc_macro2::TokenStream {
    let ident = &f.ident;
    let info = match f.ty {
        Ty::AccountInfo => quote! { #ident },
        Ty::ProgramAccount(_) => quote! { #ident.to_account_info() },
        _ => panic!("Invalid syntax: rent exemption cannot be specified."),
    };
    match c {
        ConstraintRentExempt::Skip => quote! {},
        ConstraintRentExempt::Enforce => quote! {
            if !rent.is_exempt(#info.lamports(), #info.try_data_len()?) {
                return Err(anchor_lang::solana_program::program_error::ProgramError::Custom(2)); // todo: error codes
            }
        },
    }
}

pub fn generate_constraint_seeds(f: &Field, c: &ConstraintSeeds) -> proc_macro2::TokenStream {
    let name = &f.ident;
    let seeds = &c.seeds;
    quote! {
        let program_signer = Pubkey::create_program_address(
            &#seeds,
            program_id,
        ).map_err(|_| anchor_lang::solana_program::program_error::ProgramError::Custom(1))?; // todo
        if #name.to_account_info().key != &program_signer {
            return Err(anchor_lang::solana_program::program_error::ProgramError::Custom(1)); // todo
        }
    }
}
