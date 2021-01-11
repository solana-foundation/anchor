use crate::{
    AccountsStruct, Constraint, ConstraintBelongsTo, ConstraintLiteral, ConstraintOwner,
    ConstraintRentExempt, ConstraintSigner, Field, SysvarTy, Ty,
};
use quote::quote;

pub fn generate(accs: AccountsStruct) -> proc_macro2::TokenStream {
    let acc_infos: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &Field| {
            let name = &f.ident;
            quote! {
                let #name = next_account_info(acc_infos)?;
            }
        })
        .collect();

    let (deser_fields, access_checks, return_tys) = {
        // Deserialization for each field.
        let deser_fields: Vec<proc_macro2::TokenStream> = accs
            .fields
            .iter()
            .map(generate_field_deserialization)
            .collect();
        // Constraint checks for each account fields.
        let access_checks: Vec<proc_macro2::TokenStream> = accs
            .fields
            .iter()
            .map(|f: &Field| {
                let checks: Vec<proc_macro2::TokenStream> = f
                    .constraints
                    .iter()
                    .map(|c| generate_constraint(&f, c))
                    .collect();
                quote! {
                    #(#checks)*
                }
            })
            .collect();
        // Each field in the final deserialized accounts struct.
        let return_tys: Vec<proc_macro2::TokenStream> = accs
            .fields
            .iter()
            .map(|f: &Field| {
                let name = &f.ident;
                quote! {
                    #name
                }
            })
            .collect();

        (deser_fields, access_checks, return_tys)
    };

    let on_save: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &Field| {
            let ident = &f.ident;
            let info = match f.ty {
                Ty::AccountInfo => quote! { #ident },
                Ty::ProgramAccount(_) => quote! { #ident.info },
                _ => return quote! {},
            };
            match f.is_mut {
                false => quote! {},
                true => quote! {
                    let mut data = self.#info.try_borrow_mut_data()?;
                    let dst: &mut [u8] = &mut data;
                    let mut cursor = std::io::Cursor::new(dst);
                    self.#ident.account.try_serialize(&mut cursor)?;
                },
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

    quote! {
        impl#combined_generics Accounts#trait_generics for #name#strct_generics {
            fn try_accounts(program_id: &Pubkey, accounts: &[AccountInfo<'info>]) -> Result<Self, ProgramError> {
                let acc_infos = &mut accounts.iter();

                // Pull out each account info from the `accounts` slice.
                #(#acc_infos)*

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

        impl#strct_generics #name#strct_generics {
            pub fn exit(&self) -> ProgramResult {
                #(#on_save)*
                Ok(())
            }
        }
    }
}

pub fn generate_field_deserialization(f: &Field) -> proc_macro2::TokenStream {
    let ident = &f.ident;
    let assign_ty = match &f.ty {
        Ty::AccountInfo => quote! {
            let #ident = #ident.clone();
        },
        Ty::ProgramAccount(acc) => {
            let account_struct = &acc.account_ident;
            match f.is_init {
                false => quote! {
                    let #ident: ProgramAccount<#account_struct> = ProgramAccount::try_from(#ident)?;
                },
                true => quote! {
                    let #ident: ProgramAccount<#account_struct> = ProgramAccount::try_from_init(#ident)?;
                },
            }
        }
        Ty::Sysvar(sysvar) => match sysvar {
            SysvarTy::Clock => quote! {
                let #ident = Clock::from_account_info(#ident)?;
            },
            SysvarTy::Rent => quote! {
                let #ident = Rent::from_account_info(#ident)?;
            },
            SysvarTy::EpochSchedule => quote! {
                let #ident = EpochSchedule::from_account_info(#ident)?;
            },
            SysvarTy::Fees => quote! {
                let #ident = Fees::from_account_info(#ident)?;
            },
            SysvarTy::RecentBlockHashes => quote! {
                let #ident = RecentBlockhashes::from_account_info(#ident)?;
            },
            SysvarTy::SlotHashes => quote! {
                let #ident = SlotHashes::from_account_info(#ident)?;
            },
            SysvarTy::SlotHistory => quote! {
                let #ident = SlotHistory::from_account_info(#ident)?;
            },
            SysvarTy::StakeHistory => quote! {
                let #ident = StakeHistory::from_account_info(#ident)?;
            },
            SysvarTy::Instructions => quote! {
                let #ident = Instructions::from_account_info(#ident)?;
            },
            SysvarTy::Rewards => quote! {
                let #ident = Rewards::from_account_info(#ident)?;
            },
        },
    };

    quote! {
        #assign_ty
    }
}

pub fn generate_constraint(f: &Field, c: &Constraint) -> proc_macro2::TokenStream {
    match c {
        Constraint::BelongsTo(c) => generate_constraint_belongs_to(f, c),
        Constraint::Signer(c) => generate_constraint_signer(f, c),
        Constraint::Literal(c) => generate_constraint_literal(f, c),
        Constraint::Owner(c) => generate_constraint_owner(f, c),
        Constraint::RentExempt(c) => generate_constraint_rent_exempt(f, c),
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
        if &#ident.#target != #target.info.key {
            return Err(ProgramError::Custom(1)); // todo: error codes
        }
    }
}

pub fn generate_constraint_signer(f: &Field, _c: &ConstraintSigner) -> proc_macro2::TokenStream {
    let ident = &f.ident;
    let info = match f.ty {
        Ty::AccountInfo => quote! { #ident },
        Ty::ProgramAccount(_) => quote! { #ident.info },
        _ => panic!("Invalid syntax: signer cannot be specified."),
    };
    quote! {
        if !#info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
    }
}

pub fn generate_constraint_literal(_f: &Field, c: &ConstraintLiteral) -> proc_macro2::TokenStream {
    let tokens = &c.tokens;
    quote! {
        if !(#tokens) {
            return Err(ProgramError::Custom(1)); // todo: error codes
        }
    }
}

pub fn generate_constraint_owner(f: &Field, c: &ConstraintOwner) -> proc_macro2::TokenStream {
    let ident = &f.ident;
    let info = match f.ty {
        Ty::AccountInfo => quote! { #ident },
        Ty::ProgramAccount(_) => quote! { #ident.info },
        _ => panic!("Invalid syntax: owner cannot be specified."),
    };
    match c {
        ConstraintOwner::Skip => quote! {},
        ConstraintOwner::Program => quote! {
            if #info.owner != program_id {
                return Err(ProgramError::Custom(1)); // todo: error codes
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
        Ty::ProgramAccount(_) => quote! { #ident.info },
        _ => panic!("Invalid syntax: rent exemption cannot be specified."),
    };
    match c {
        ConstraintRentExempt::Skip => quote! {},
        ConstraintRentExempt::Enforce => quote! {
            if !rent.is_exempt(#info.lamports(), #info.try_data_len()?) {
                return Err(ProgramError::Custom(2)); // todo: error codes
            }
        },
    }
}
