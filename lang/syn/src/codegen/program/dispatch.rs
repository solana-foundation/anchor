use crate::codegen::program::common::*;
use crate::{Program, State};
use heck::CamelCase;
use quote::quote;

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    // Dispatch the state constructor.
    let ctor_state_dispatch_arm = match &program.state {
        None => quote! { /* no-op */ },
        Some(state) => match state.ctor_and_anchor.is_some() {
            false => quote! {},
            true => {
                let variant_arm = generate_ctor_variant(state);
                let ctor_args = generate_ctor_args(state);
                let ix_name: proc_macro2::TokenStream =
                    generate_ctor_variant_name().parse().unwrap();
                let sighash_arr = sighash_ctor();
                let sighash_tts: proc_macro2::TokenStream =
                    format!("{:?}", sighash_arr).parse().unwrap();
                quote! {
                    #sighash_tts => {
                        let ix = instruction::state::#ix_name::deserialize(&mut ix_data)
                            .map_err(|_| ProgramError::Custom(1))?; // todo: error code
                        let instruction::state::#variant_arm = ix;
                        __private::__state::__ctor(program_id, accounts, #(#ctor_args),*)
                    }
                }
            }
        },
    };

    // Dispatch the state impl instructions.
    let state_dispatch_arms: Vec<proc_macro2::TokenStream> = match &program.state {
        None => vec![],
        Some(s) => s
            .impl_block_and_methods
            .as_ref()
            .map(|(_impl_block, methods)| {
                methods
                    .iter()
                    .map(|ix: &crate::StateIx| {
                        let ix_arg_names: Vec<&syn::Ident> =
                            ix.args.iter().map(|arg| &arg.name).collect();
                        let name = &ix.raw_method.sig.ident.to_string();
                        let ix_method_name: proc_macro2::TokenStream =
                        { format!("__{}", name).parse().unwrap() };
                        let variant_arm =
                            generate_ix_variant(ix.raw_method.sig.ident.to_string(), &ix.args);
                        let ix_name = generate_ix_variant_name(ix.raw_method.sig.ident.to_string());
                        let sighash_arr = sighash(SIGHASH_STATE_NAMESPACE, &name);
                        let sighash_tts: proc_macro2::TokenStream =
                            format!("{:?}", sighash_arr).parse().unwrap();
                        quote! {
                            #sighash_tts => {
                                let ix = instruction::state::#ix_name::deserialize(&mut ix_data)
                                    .map_err(|_| ProgramError::Custom(1))?; // todo: error code
                                let instruction::state::#variant_arm = ix;
                                __private::__state::#ix_method_name(program_id, accounts, #(#ix_arg_names),*)
                            }
                        }
                    })
                    .collect()
            })
            .unwrap_or_default(),
    };

    // Dispatch all trait interface implementations.
    let trait_dispatch_arms: Vec<proc_macro2::TokenStream> = match &program.state {
        None => vec![],
        Some(s) => s
            .interfaces
            .as_ref()
            .map(|interfaces| {
                interfaces
                    .iter()
                    .flat_map(|iface: &crate::StateInterface| {
                        iface
                            .methods
                            .iter()
                            .map(|m: &crate::StateIx| {
                                let ix_arg_names: Vec<&syn::Ident> =
                                    m.args.iter().map(|arg| &arg.name).collect();
                                let name = &m.raw_method.sig.ident.to_string();
                                let ix_name: proc_macro2::TokenStream =  format!("__{}_{}", iface.trait_name, name).parse().unwrap();
                                let raw_args: Vec<&syn::PatType> = m
                                    .args
                                    .iter()
                                    .map(|arg: &crate::IxArg| &arg.raw_arg)
                                    .collect();
                                let sighash_arr = sighash(&iface.trait_name, &m.ident.to_string());
                                let sighash_tts: proc_macro2::TokenStream =
                                    format!("{:?}", sighash_arr).parse().unwrap();
                                let args_struct = {
                                    if m.args.is_empty() {
                                        quote! {
                                            #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
                                            struct Args;
                                        }
                                    } else {
                                        quote! {
                                            #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
                                            struct Args {
                                                #(#raw_args),*
                                            }
                                        }
                                    }
                                };
                                quote! {
                                    #sighash_tts => {
                                        #args_struct
                                        let ix = Args::deserialize(&mut ix_data)
                                            .map_err(|_| ProgramError::Custom(1))?; // todo: error code
                                        let Args {
                                            #(#ix_arg_names),*
                                        } = ix;
                                        __private::__interface::#ix_name(program_id, accounts, #(#ix_arg_names),*)
                                    }
                                }
                            })
                            .collect::<Vec<proc_macro2::TokenStream>>()
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    // Dispatch all global instructions.
    let global_dispatch_arms: Vec<proc_macro2::TokenStream> = program
        .ixs
        .iter()
        .map(|ix| {
            let ix_arg_names: Vec<&syn::Ident> = ix.args.iter().map(|arg| &arg.name).collect();
            let ix_method_name = &ix.raw_method.sig.ident;
            let ix_name = generate_ix_variant_name(ix.raw_method.sig.ident.to_string());
            let sighash_arr = sighash(SIGHASH_GLOBAL_NAMESPACE, &ix_method_name.to_string());
            let sighash_tts: proc_macro2::TokenStream =
                format!("{:?}", sighash_arr).parse().unwrap();
            let variant_arm = generate_ix_variant(ix.raw_method.sig.ident.to_string(), &ix.args);
            quote! {
                #sighash_tts => {
                    let ix = instruction::#ix_name::deserialize(&mut ix_data)
                        .map_err(|_| ProgramError::Custom(1))?; // todo: error code
                    let instruction::#variant_arm = ix;
                    __private::__global::#ix_method_name(program_id, accounts, #(#ix_arg_names),*)
                }
            }
        })
        .collect();

    quote! {
        /// Performs method dispatch.
        ///
        /// Each method in an anchor program is uniquely defined by a namespace
        /// and a rust identifier (i.e., the name given to the method). These
        /// two pieces can be combined to creater a method identifier,
        /// specifically, Anchor uses
        ///
        /// Sha256("<namespace>::<rust-identifier>")[..8],
        ///
        /// where the namespace can be one of three types. 1) "global" for a
        /// regular instruction, 2) "state" for a state struct instruction
        /// handler and 3) a trait namespace (used in combination with the
        /// `#[interface]` attribute), which is defined by the trait name, e..
        /// `MyTrait`.
        ///
        /// With this 8 byte identifier, Anchor performs method dispatch,
        /// matching the given 8 byte identifier to the associated method
        /// handler, which leads to user defined code being eventually invoked.
        fn dispatch(program_id: &Pubkey, accounts: &[AccountInfo], sighash: [u8; 8], mut ix_data: &[u8]) -> ProgramResult {
            // If the method identifier is the IDL tag, then execute an IDL
            // instruction, injected into all Anchor programs.
            if cfg!(not(feature = "no-idl")) {
                if sighash == anchor_lang::idl::IDL_IX_TAG.to_le_bytes() {
                    return __private::__idl::__idl_dispatch(program_id, accounts, &ix_data);
                }
            }

            match sighash {
                #ctor_state_dispatch_arm
                #(#state_dispatch_arms)*
                #(#trait_dispatch_arms)*
                #(#global_dispatch_arms)*
                _ => {
                    msg!("Fallback functions are not supported. If you have a use case, please file an issue.");
                    Err(ProgramError::Custom(99))
                }
            }
        }
    }
}

fn generate_ctor_variant_name() -> String {
    "New".to_string()
}

fn generate_ctor_variant(state: &State) -> proc_macro2::TokenStream {
    let ctor_args = generate_ctor_args(state);
    let ctor_variant_name: proc_macro2::TokenStream = generate_ctor_variant_name().parse().unwrap();
    if ctor_args.is_empty() {
        quote! {
            #ctor_variant_name
        }
    } else {
        quote! {
            #ctor_variant_name {
                #(#ctor_args),*
            }
        }
    }
}

fn generate_ix_variant_name(name: String) -> proc_macro2::TokenStream {
    let n = name.to_camel_case();
    n.parse().unwrap()
}
