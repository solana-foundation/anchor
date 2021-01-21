use crate::{Program, Rpc};
use heck::CamelCase;
use quote::quote;

pub fn generate(program: Program) -> proc_macro2::TokenStream {
    let mod_name = &program.name;
    let instruction_name = instruction_enum_name(&program);
    let dispatch = generate_dispatch(&program);
    let handlers_non_inlined = generate_non_inlined_handlers(&program);
    let methods = generate_methods(&program);
    let instruction = generate_instruction(&program);
    let cpi = generate_cpi(&program);

    quote! {
        // Import everything in the mod, in case the user wants to put types
        // in there.
        use #mod_name::*;

        #[cfg(not(feature = "no-entrypoint"))]
        anchor_lang::solana_program::entrypoint!(entry);
        #[cfg(not(feature = "no-entrypoint"))]
        fn entry(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
            let mut data: &[u8] = instruction_data;
            let ix = __private::instruction::#instruction_name::deserialize(&mut data)
                .map_err(|_| ProgramError::Custom(1))?; // todo: error code

                #dispatch
        }

        // Create a private module to not clutter the program's namespace.
        mod __private {
            use super::*;

            #handlers_non_inlined

            #instruction
        }

        #methods

        #cpi
    }
}
pub fn generate_dispatch(program: &Program) -> proc_macro2::TokenStream {
    let dispatch_arms: Vec<proc_macro2::TokenStream> = program
        .rpcs
        .iter()
        .map(|rpc| {
            let rpc_arg_names: Vec<&syn::Ident> = rpc.args.iter().map(|arg| &arg.name).collect();
            let variant_arm = generate_ix_variant(program, rpc);
            let rpc_name = &rpc.raw_method.sig.ident;
            quote! {
                __private::instruction::#variant_arm => {
                    __private::#rpc_name(program_id, accounts, #(#rpc_arg_names),*)
                }
            }
        })
        .collect();

    quote! {
        match ix {
            #(#dispatch_arms),*
        }
    }
}

// Generate non-inlined wrappers for each instruction handler, since Solana's
// BPF max stack size can't handle reasonable sized dispatch trees without doing
// so.
pub fn generate_non_inlined_handlers(program: &Program) -> proc_macro2::TokenStream {
    let program_name = &program.name;
    let non_inlined_handlers: Vec<proc_macro2::TokenStream> = program
        .rpcs
        .iter()
        .map(|rpc| {
            let rpc_params: Vec<_> = rpc.args.iter().map(|arg| &arg.raw_arg).collect();
            let rpc_arg_names: Vec<&syn::Ident> = rpc.args.iter().map(|arg| &arg.name).collect();
            let rpc_name = &rpc.raw_method.sig.ident;
            let anchor = &rpc.anchor_ident;

            quote! {
                #[inline(never)]
                pub fn #rpc_name(
                    program_id: &Pubkey,
                    accounts: &[AccountInfo],
                    #(#rpc_params),*
                ) -> ProgramResult {
                    let mut remaining_accounts: &[AccountInfo] = accounts;
                    let mut accounts = #anchor::try_accounts(program_id, &mut remaining_accounts)?;
                    #program_name::#rpc_name(
                        Context::new(program_id, &mut accounts, remaining_accounts),
                        #(#rpc_arg_names),*
                    )?;
                    accounts.exit(program_id)
                }
            }
        })
        .collect();

    quote! {
        #(#non_inlined_handlers)*
    }
}

pub fn generate_ix_variant(program: &Program, rpc: &Rpc) -> proc_macro2::TokenStream {
    let enum_name = instruction_enum_name(program);
    let rpc_arg_names: Vec<&syn::Ident> = rpc.args.iter().map(|arg| &arg.name).collect();
    let rpc_name_camel = proc_macro2::Ident::new(
        &rpc.raw_method.sig.ident.to_string().to_camel_case(),
        rpc.raw_method.sig.ident.span(),
    );
    if rpc.args.len() == 0 {
        quote! {
            #enum_name::#rpc_name_camel
        }
    } else {
        quote! {
            #enum_name::#rpc_name_camel {
                #(#rpc_arg_names),*
            }
        }
    }
}

pub fn generate_methods(program: &Program) -> proc_macro2::TokenStream {
    let program_mod = &program.program_mod;
    quote! {
        #program_mod
    }
}

pub fn generate_instruction(program: &Program) -> proc_macro2::TokenStream {
    let enum_name = instruction_enum_name(program);
    let variants: Vec<proc_macro2::TokenStream> = program
        .rpcs
        .iter()
        .map(|rpc| {
            let rpc_name_camel = proc_macro2::Ident::new(
                &rpc.raw_method.sig.ident.to_string().to_camel_case(),
                rpc.raw_method.sig.ident.span(),
            );
            let raw_args: Vec<&syn::PatType> = rpc.args.iter().map(|arg| &arg.raw_arg).collect();
            // If no args, output a "unit" variant instead of a struct variant.
            if rpc.args.len() == 0 {
                quote! {
                    #rpc_name_camel
                }
            } else {
                quote! {
                    #rpc_name_camel {
                        #(#raw_args),*
                    }
                }
            }
        })
        .collect();

    quote! {
        pub mod instruction {
            use super::*;
            #[derive(AnchorSerialize, AnchorDeserialize)]
            pub enum #enum_name {
                #(#variants),*
            }
        }
    }
}

fn instruction_enum_name(program: &Program) -> proc_macro2::Ident {
    proc_macro2::Ident::new(
        &format!("{}Instruction", program.name.to_string().to_camel_case()),
        program.name.span(),
    )
}

fn generate_cpi(program: &Program) -> proc_macro2::TokenStream {
    let cpi_methods: Vec<proc_macro2::TokenStream> = program
        .rpcs
        .iter()
        .map(|rpc| {
            let accounts_ident = &rpc.anchor_ident;
            let cpi_method = {
                let ix_variant = generate_ix_variant(program, rpc);
                let method_name = &rpc.ident;
                let args: Vec<&syn::PatType> = rpc.args.iter().map(|arg| &arg.raw_arg).collect();
                quote! {
                    pub fn #method_name<'a, 'b, 'c, 'info>(
                        ctx: CpiContext<'a, 'b, 'c, 'info, #accounts_ident<'info>>,
                        #(#args),*
                    ) -> ProgramResult {
                        let ix = {
                            let ix = __private::instruction::#ix_variant;
                            let data = AnchorSerialize::try_to_vec(&ix)
                                .map_err(|_| ProgramError::InvalidInstructionData)?;
                            let accounts = ctx.accounts.to_account_metas(None);
                            anchor_lang::solana_program::instruction::Instruction {
                                program_id: *ctx.program.key,
                                accounts,
                                data,
                            }
                        };
                        let mut acc_infos = ctx.accounts.to_account_infos();
                        acc_infos.push(ctx.program.clone());
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
    quote! {
        #[cfg(feature = "cpi")]
        pub mod cpi {
            use super::*;

            #(#cpi_methods)*
        }
    }
}
