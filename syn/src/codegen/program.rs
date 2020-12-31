use crate::Program;
use heck::CamelCase;
use quote::quote;

pub fn generate(program: Program) -> proc_macro2::TokenStream {
    let mod_name = &program.name;
    let instruction_name = instruction_enum_name(&program);
    let dispatch = generate_dispatch(&program);
    let methods = generate_methods(&program);
    let instruction = generate_instruction(&program);

    quote! {
        // Import everything in the mod, in case the user wants to put anchors
        // in there.
        use #mod_name::*;

        solana_program::entrypoint!(entry);
        fn entry(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
            let mut data: &[u8] = instruction_data;
            let ix = instruction::#instruction_name::deserialize(&mut data)
                .map_err(|_| ProgramError::Custom(1))?; // todo: error code

                #dispatch
        }

        #methods

        #instruction
    }
}
pub fn generate_dispatch(program: &Program) -> proc_macro2::TokenStream {
    let program_name = &program.name;
    let enum_name = instruction_enum_name(program);
    let dispatch_arms: Vec<proc_macro2::TokenStream> = program
        .rpcs
        .iter()
        .map(|rpc| {
            let rpc_arg_names: Vec<&syn::Ident> = rpc.args.iter().map(|arg| &arg.name).collect();

            let variant_arm = {
                let rpc_name_camel = proc_macro2::Ident::new(
                    &rpc.raw_method.sig.ident.to_string().to_camel_case(),
                    rpc.raw_method.sig.ident.span(),
                );
                // If no args, output a "unit" variant instead of a struct variant.
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
            };

            let rpc_name = &rpc.raw_method.sig.ident;
            let anchor = &rpc.anchor_ident;

            quote! {
                instruction::#variant_arm => {
                    let mut accounts = #anchor::try_anchor(program_id, accounts)?;
                    #program_name::#rpc_name(
                        Context {
                            accounts: &mut accounts,
                            program_id,
                        },
                        #(#rpc_arg_names),*
                    )?;
                    accounts.exit()
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
        &program.name.to_string().to_camel_case(),
        program.name.span(),
    )
}
