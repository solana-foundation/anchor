use crate::{
    AccountsStruct, Constraint, ConstraintBelongsTo, ConstraintLiteral, ConstraintOwner,
    ConstraintSigner, Field, Ty,
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

    let (access_checks, return_tys): (
        Vec<proc_macro2::TokenStream>,
        Vec<proc_macro2::TokenStream>,
    ) = accs
        .fields
        .iter()
        .map(|f: &Field| {
            let name = &f.ident;

            // Account validation.
            let access_control = generate_field(f);

            // Single field in the final deserialized accounts struct.
            let return_ty = quote! {
                #name
            };

            (access_control, return_ty)
        })
        .unzip();

    let on_save: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &Field| {
            let ident = &f.ident;
            let info = match f.ty {
                Ty::AccountInfo => quote! { #ident },
                Ty::ProgramAccount(_) => quote! { #ident.info },
            };
            match f.is_mut {
                false => quote! {},
                true => quote! {
                        let mut data = self.#info.try_borrow_mut_data()?;
                        let dst: &mut [u8] = &mut data;
                        let mut cursor = std::io::Cursor::new(dst);
                        self.#ident.account.serialize(&mut cursor)
                                .map_err(|_| ProgramError::InvalidAccountData)?;
                },
            }
        })
        .collect();

    let name = &accs.ident;
    let generics = &accs.generics;

    quote! {
        impl#generics Accounts#generics for #name#generics {
            fn try_anchor(program_id: &Pubkey, accounts: &[AccountInfo<'info>]) -> Result<Self, ProgramError> {
                let acc_infos = &mut accounts.iter();

                #(#acc_infos)*

                #(#access_checks)*

                Ok(#name {
                    #(#return_tys),*
                })
            }
        }

        impl#generics #name#generics {
            pub fn exit(&self) -> ProgramResult {
                #(#on_save)*
                Ok(())
            }
        }
    }
}

// Unpacks the field, if needed.
pub fn generate_field(f: &Field) -> proc_macro2::TokenStream {
    let checks: Vec<proc_macro2::TokenStream> = f
        .constraints
        .iter()
        .map(|c| generate_constraint(&f, c))
        .collect();
    let ident = &f.ident;
    let assign_ty = match &f.ty {
        Ty::AccountInfo => quote! {
            let #ident = #ident.clone();
        },
        Ty::ProgramAccount(acc) => {
            let account_struct = &acc.account_ident;
            quote! {
                let mut data: &[u8] = &#ident.try_borrow_data()?;
                let #ident = ProgramAccount::new(
                    #ident.clone(),
                    #account_struct::deserialize(&mut data)
                    .map_err(|_| ProgramError::InvalidAccountData)?
                );
            }
        }
    };
    quote! {
        #assign_ty
        #(#checks)*
    }
}

pub fn generate_constraint(f: &Field, c: &Constraint) -> proc_macro2::TokenStream {
    match c {
        Constraint::BelongsTo(c) => generate_constraint_belongs_to(f, c),
        Constraint::Signer(c) => generate_constraint_signer(f, c),
        Constraint::Literal(c) => generate_constraint_literal(f, c),
        Constraint::Owner(c) => generate_constraint_owner(f, c),
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
        if #tokens {
            return Err(ProgramError::Custom(1)); // todo: error codes
        }
    }
}

pub fn generate_constraint_owner(f: &Field, c: &ConstraintOwner) -> proc_macro2::TokenStream {
    let ident = &f.ident;
    let info = match f.ty {
        Ty::AccountInfo => quote! { #ident },
        Ty::ProgramAccount(_) => quote! { #ident.info },
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
