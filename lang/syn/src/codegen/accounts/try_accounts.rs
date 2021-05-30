use crate::codegen::accounts::{constraints, generics};
use crate::{AccountField, AccountsStruct, Field, SysvarTy, Ty};
use proc_macro2::TokenStream;
use quote::quote;

// Generates the `Accounts` trait implementation.
pub fn generate(accs: &AccountsStruct) -> proc_macro2::TokenStream {
    let name = &accs.ident;
    let (combined_generics, trait_generics, strct_generics) = generics(accs);

    // All fields without an `#[account(associated)]` attribute.
    let non_associated_fields: Vec<&AccountField> = accs
        .fields
        .iter()
        .filter(|af| !is_associated_init(af))
        .collect();

    // Deserialization for each field
    let deser_fields: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|af: &AccountField| {
            match af {
                AccountField::CompositeField(s) => {
                    let name = &s.ident;
                    let ty = &s.raw_field.ty;
                    quote! {
                        #[cfg(feature = "anchor-debug")]
                        ::solana_program::log::sol_log(stringify!(#name));
                        let #name: #ty = anchor_lang::Accounts::try_accounts(program_id, accounts)?;
                    }
                }
                AccountField::Field(f) => {
                    // Associated fields are *first* deserialized into
                    // AccountInfos, and then later deserialized into
                    // ProgramAccounts in the "constraint check" phase.
                    if is_associated_init(af) {
                        let name = &f.ident;
                        quote!{
                            let #name = &accounts[0];
                            *accounts = &accounts[1..];
                        }
                    } else {
                        let name = typed_ident(&f);
                        match f.constraints.is_init() {
                            false => quote! {
                                #[cfg(feature = "anchor-debug")]
                                ::solana_program::log::sol_log(stringify!(#name));
                                let #name = anchor_lang::Accounts::try_accounts(program_id, accounts)?;
                            },
                            true => quote! {
                                #[cfg(feature = "anchor-debug")]
                                ::solana_program::log::sol_log(stringify!(#name));
                                let #name = anchor_lang::AccountsInit::try_accounts_init(program_id, accounts)?;
                            },
                        }
                    }
                }
            }
        })
        .collect();

    // Deserialization for each *associated* field. This must be after
    // the deser_fields.
    let deser_associated_fields: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .filter_map(|af| match af {
            AccountField::CompositeField(_s) => None,
            AccountField::Field(f) => match is_associated_init(af) {
                false => None,
                true => Some(f),
            },
        })
        .map(|field: &Field| constraints::generate(field))
        .collect();

    // Constraint checks for each account fields.
    let access_checks: Vec<proc_macro2::TokenStream> = non_associated_fields
        .iter()
        .map(|af: &&AccountField| match af {
            AccountField::Field(f) => constraints::generate(f),
            AccountField::CompositeField(s) => constraints::generate_composite(s),
        })
        .collect();

    // Each field in the final deserialized accounts struct.
    let return_tys: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &AccountField| {
            let name = match f {
                AccountField::CompositeField(s) => &s.ident,
                AccountField::Field(f) => &f.ident,
            };
            quote! {
                #name
            }
        })
        .collect();
    quote! {
        impl#combined_generics anchor_lang::Accounts#trait_generics for #name#strct_generics {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
                // Deserialize each account.
                #(#deser_fields)*
                // Deserialize each associated account.
                //
                // Associated accounts are treated specially, because the fields
                // do deserialization + constraint checks in a single go,
                // whereas all other fields, i.e. the `deser_fields`, first
                // deserialize, and then do constraint checks.
                #(#deser_associated_fields)*
                // Perform constraint checks on each account.
                #(#access_checks)*
                // Success. Return the validated accounts.
                Ok(#name {
                    #(#return_tys),*
                })
            }
        }
    }
}

// Returns true if the given AccountField has an associated init constraint.
fn is_associated_init(af: &AccountField) -> bool {
    match af {
        AccountField::CompositeField(_s) => false,
        AccountField::Field(f) => f
            .constraints
            .associated
            .as_ref()
            .map(|f| f.is_init)
            .unwrap_or(false),
    }
}

fn typed_ident(field: &Field) -> TokenStream {
    let name = &field.ident;

    let ty = match &field.ty {
        Ty::AccountInfo => quote! { AccountInfo },
        Ty::ProgramState(ty) => {
            let account = &ty.account_ident;
            quote! {
                ProgramState<#account>
            }
        }
        Ty::CpiState(ty) => {
            let account = &ty.account_ident;
            quote! {
                CpiState<#account>
            }
        }
        Ty::ProgramAccount(ty) => {
            let account = &ty.account_ident;
            quote! {
                ProgramAccount<#account>
            }
        }
        Ty::Loader(ty) => {
            let account = &ty.account_ident;
            quote! {
                Loader<#account>
            }
        }
        Ty::CpiAccount(ty) => {
            let account = &ty.account_ident;
            quote! {
                CpiAccount<#account>
            }
        }
        Ty::Sysvar(ty) => {
            let account = match ty {
                SysvarTy::Clock => quote! {Clock},
                SysvarTy::Rent => quote! {Rent},
                SysvarTy::EpochSchedule => quote! {EpochSchedule},
                SysvarTy::Fees => quote! {Fees},
                SysvarTy::RecentBlockhashes => quote! {RecentBlockhashes},
                SysvarTy::SlotHashes => quote! {SlotHashes},
                SysvarTy::SlotHistory => quote! {SlotHistory},
                SysvarTy::StakeHistory => quote! {StakeHistory},
                SysvarTy::Instructions => quote! {Instructions},
                SysvarTy::Rewards => quote! {Rewards},
            };
            quote! {
                Sysvar<#account>
            }
        }
    };

    quote! {
        #name: #ty
    }
}
