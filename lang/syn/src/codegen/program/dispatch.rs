use crate::codegen::program::common::*;
use crate::Program;
use quote::quote;

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    // Dispatch the state constructor.
    let ctor_state_dispatch_arm = match &program.state {
        None => quote! { /* no-op */ },
        Some(state) => match state.ctor_and_anchor.is_some() {
            false => quote! {},
            true => {
                let sighash_arr = sighash_ctor();
                let sighash_tts: proc_macro2::TokenStream =
                    format!("{:?}", sighash_arr).parse().unwrap();
                quote! {
                    #sighash_tts => {
                        __private::__state::__ctor(
                            program_id,
                            accounts,
                            ix_data,
                        )
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
                        let name = &ix.raw_method.sig.ident.to_string();
                        let ix_method_name: proc_macro2::TokenStream =
                            { format!("__{}", name).parse().unwrap() };
                        let sighash_arr = sighash(SIGHASH_STATE_NAMESPACE, name);
                        let sighash_tts: proc_macro2::TokenStream =
                            format!("{:?}", sighash_arr).parse().unwrap();
                        quote! {
                            #sighash_tts => {
                                __private::__state::#ix_method_name(
                                    program_id,
                                    accounts,
                                    ix_data,
                                )
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
                                let sighash_arr = sighash(&iface.trait_name, &m.ident.to_string());
                                let sighash_tts: proc_macro2::TokenStream =
                                    format!("{:?}", sighash_arr).parse().unwrap();
                                let name = &m.raw_method.sig.ident.to_string();
                                let ix_method_name: proc_macro2::TokenStream =
                                    format!("__{}_{}", iface.trait_name, name).parse().unwrap();
                                quote! {
                                    #sighash_tts => {
                                        __private::__interface::#ix_method_name(
                                            program_id,
                                            accounts,
                                            ix_data,
                                        )
                                    }
                                }
                            })
                            .collect::<Vec<proc_macro2::TokenStream>>()
                    })
                    .collect()
            })
            .unwrap_or_default(),
    };

    // Dispatch all global instructions.
    let global_dispatch_arms: Vec<proc_macro2::TokenStream> = program
        .ixs
        .iter()
        .map(|ix| {
            let ix_method_name = &ix.raw_method.sig.ident;
            let sighash_arr = sighash(SIGHASH_GLOBAL_NAMESPACE, &ix_method_name.to_string());
            let sighash_tts: proc_macro2::TokenStream =
                format!("{:?}", sighash_arr).parse().unwrap();
            quote! {
                #sighash_tts => {
                    __private::__global::#ix_method_name(
                        program_id,
                        accounts,
                        ix_data,
                    )
                }
            }
        })
        .collect();
    let fallback_fn = gen_fallback(program).unwrap_or(quote! {
        Err(anchor_lang::error::ErrorCode::InstructionFallbackNotFound.into())
    });
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
        fn dispatch(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            data: &[u8],
        ) -> anchor_lang::Result<()> {
            // Split the instruction data into the first 8 byte method
            // identifier (sighash) and the serialized instruction data.
            let mut ix_data: &[u8] = data;
            let sighash: [u8; 8] = {
                let mut sighash: [u8; 8] = [0; 8];
                sighash.copy_from_slice(&ix_data[..8]);
                ix_data = &ix_data[8..];
                sighash
            };

            // If the method identifier is the IDL tag, then execute an IDL
            // instruction, injected into all Anchor programs.
            if cfg!(not(feature = "no-idl")) {
                if sighash == anchor_lang::idl::IDL_IX_TAG.to_le_bytes() {
                    return __private::__idl::__idl_dispatch(
                        program_id,
                        accounts,
                        &ix_data,
                    );
                }
            }

            match sighash {
                #ctor_state_dispatch_arm
                #(#state_dispatch_arms)*
                #(#trait_dispatch_arms)*
                #(#global_dispatch_arms)*
                _ => {
                    #fallback_fn
                }
            }
        }
    }
}

pub fn gen_fallback(program: &Program) -> Option<proc_macro2::TokenStream> {
    program.fallback_fn.as_ref().map(|fallback_fn| {
        let program_name = &program.name;
        let method = &fallback_fn.raw_method;
        let fn_name = &method.sig.ident;
        quote! {
            #program_name::#fn_name(program_id, accounts, data)
        }
    })
}
