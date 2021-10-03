use crate::codegen::program::common::{generate_ix_variant, sighash, SIGHASH_GLOBAL_NAMESPACE};
use crate::Program;
use crate::StateIx;
use heck::SnakeCase;
use quote::quote;

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    // Generate cpi methods for the state struct.
    // The Ctor is not exposed via CPI, since it is a one time use function.
    let state_cpi_methods: Vec<proc_macro2::TokenStream> = program
        .state
        .as_ref()
        .map(|state| {
            state
                .impl_block_and_methods
                .as_ref()
                .map(|(_, methods)| {
                    methods
                        .iter()
                        .map(|method: &StateIx| {
                            let accounts_ident = &method.anchor_ident;
                            let ix_variant = generate_ix_variant(
                                method.raw_method.sig.ident.to_string(),
                                &method.args,
                            );
                            let method_name = &method.ident;
                            let args: Vec<&syn::PatType> =
                                method.args.iter().map(|arg| &arg.raw_arg).collect();

                            quote! {
                                pub fn #method_name<'a, 'b, 'c, 'info>(
                                    ctx: CpiStateContext<'a, 'b, 'c, 'info, #accounts_ident<'info>>,
                                    #(#args),*
                                ) -> ProgramResult {
                                    let ix = {
                                        let ix = instruction::state::#ix_variant;
                                        let data = anchor_lang::InstructionData::data(&ix);
                                        let accounts = ctx.to_account_metas(None);
                                        anchor_lang::solana_program::instruction::Instruction {
                                            program_id: crate::ID,
                                            accounts,
                                            data,
                                        }
                                    };
                                    let mut acc_infos = ctx.to_account_infos();
                                    anchor_lang::solana_program::program::invoke_signed(
                                        &ix,
                                        &acc_infos,
                                        ctx.signer_seeds(),
                                    )
                                }
                            }
                        })
                        .collect()
                })
                .unwrap_or_else(Vec::new)
        })
        .unwrap_or_else(Vec::new);
    // Generate cpi methods for global methods.
    let global_cpi_methods: Vec<proc_macro2::TokenStream> = program
        .ixs
        .iter()
        .map(|ix| {
            let accounts_ident: proc_macro2::TokenStream = format!("crate::cpi::accounts::{}", &ix.anchor_ident.to_string()).parse().unwrap();
            let cpi_method = {
                let ix_variant = generate_ix_variant(ix.raw_method.sig.ident.to_string(), &ix.args);
                let method_name = &ix.ident;
                let args: Vec<&syn::PatType> = ix.args.iter().map(|arg| &arg.raw_arg).collect();
                let name = &ix.raw_method.sig.ident.to_string();
                let sighash_arr = sighash(SIGHASH_GLOBAL_NAMESPACE, name);
                let sighash_tts: proc_macro2::TokenStream =
                    format!("{:?}", sighash_arr).parse().unwrap();
                quote! {
                    pub fn #method_name<'a, 'b, 'c, 'info>(
                        ctx: CpiContext<'a, 'b, 'c, 'info, #accounts_ident<'info>>,
                        #(#args),*
                    ) -> ProgramResult {
                        let ix = {
                            let ix = instruction::#ix_variant;
                            let mut ix_data = AnchorSerialize::try_to_vec(&ix)
                                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotSerialize)?;
                            let mut data = #sighash_tts.to_vec();
                            data.append(&mut ix_data);
                            let accounts = ctx.to_account_metas(None);
                            anchor_lang::solana_program::instruction::Instruction {
                                program_id: crate::ID,
                                accounts,
                                data,
                            }
                        };
                        let mut acc_infos = ctx.to_account_infos();
                        anchor_lang::solana_program::program::invoke_signed(
                            &ix,
                            &acc_infos,
                            ctx.signer_seeds,
                        )
                    }
                }
            };

            cpi_method
        })
        .collect();

    let accounts = generate_accounts(program);

    quote! {
        #[cfg(feature = "cpi")]
        pub mod cpi {
            use super::*;

            pub mod state {
                use super::*;

                #(#state_cpi_methods)*
            }

            #(#global_cpi_methods)*

            #accounts
        }
    }
}

pub fn generate_accounts(program: &Program) -> proc_macro2::TokenStream {
    let mut accounts = std::collections::HashSet::new();

    // Go through state accounts.
    if let Some(state) = &program.state {
        // Ctor.
        if let Some((_ctor, ctor_accounts)) = &state.ctor_and_anchor {
            let macro_name = format!(
                "__cpi_client_accounts_{}",
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
                    "__cpi_client_accounts_{}",
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
            "__cpi_client_accounts_{}",
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

    quote! {
        /// An Anchor generated module, providing a set of structs
        /// mirroring the structs deriving `Accounts`, where each field is
        /// an `AccountInfo`. This is useful for CPI.
        pub mod accounts {
            #(#account_structs)*
        }
    }
}
