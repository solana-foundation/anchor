use crate::codegen::program::common::*;
use crate::parser;
use crate::Program;
use heck::CamelCase;
use quote::quote;

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    let ctor_variant = match &program.state {
        None => quote! {},
        Some(state) => {
            let ctor_args: Vec<proc_macro2::TokenStream> = generate_ctor_typed_args(state)
                .iter()
                .map(|arg| {
                    format!("pub {}", parser::tts_to_string(&arg))
                        .parse()
                        .unwrap()
                })
                .collect();
            let strct = {
                if ctor_args.is_empty() {
                    quote! {
                        #[derive(AnchorSerialize, AnchorDeserialize)]
                        pub struct New;
                    }
                } else {
                    quote! {
                        #[derive(AnchorSerialize, AnchorDeserialize)]
                        pub struct New {
                            #(#ctor_args),*
                        }
                    }
                }
            };
            let sighash_arr = sighash_ctor();
            let sighash_tts: proc_macro2::TokenStream =
                format!("{:?}", sighash_arr).parse().unwrap();
            quote! {
                /// Instruction arguments to the `#[state]`'s `new`
                /// constructor.
                #strct

                impl anchor_lang::InstructionData for New {
                    fn data(&self) -> Vec<u8> {
                        let mut d = #sighash_tts.to_vec();
                        d.append(&mut self.try_to_vec().expect("Should always serialize"));
                        d
                    }
                }
            }
        }
    };
    let state_method_variants: Vec<proc_macro2::TokenStream> = match &program.state {
        None => vec![],
        Some(state) => state
            .impl_block_and_methods
            .as_ref()
            .map(|(_impl_block, methods)| {
                methods
                    .iter()
                    .map(|method| {
                        let ix_name_camel: proc_macro2::TokenStream = method
                            .raw_method
                            .sig
                            .ident
                            .to_string()
                            .to_camel_case()
                            .parse()
                            .unwrap();
                        let raw_args: Vec<proc_macro2::TokenStream> = method
                            .args
                            .iter()
                            .map(|arg| {
                                format!("pub {}", parser::tts_to_string(&arg.raw_arg))
                                    .parse()
                                    .unwrap()
                            })
                            .collect();

                        let ix_data_trait = {
                            let name = method.raw_method.sig.ident.to_string();
                            let sighash_arr = sighash(SIGHASH_STATE_NAMESPACE, &name);
                            let sighash_tts: proc_macro2::TokenStream =
                                format!("{:?}", sighash_arr).parse().unwrap();
                            quote! {
                                impl anchor_lang::InstructionData for #ix_name_camel {
                                    fn data(&self) -> Vec<u8> {
                                        let mut d = #sighash_tts.to_vec();
                                        d.append(&mut self.try_to_vec().expect("Should always serialize"));
                                        d
                                    }
                                }
                            }
                        };

                        // If no args, output a "unit" variant instead of a struct variant.
                        if method.args.is_empty() {
                            quote! {
                                /// Anchor generated instruction.
                                #[derive(AnchorSerialize, AnchorDeserialize)]
                                pub struct #ix_name_camel;

                                #ix_data_trait
                            }
                        } else {
                            quote! {
                                /// Anchor generated instruction.
                                #[derive(AnchorSerialize, AnchorDeserialize)]
                                pub struct #ix_name_camel {
                                    #(#raw_args),*
                                }

                                #ix_data_trait
                            }
                        }
                    })
                    .collect()
            })
            .unwrap_or_default(),
    };
    let variants: Vec<proc_macro2::TokenStream> = program
        .ixs
        .iter()
        .map(|ix| {
            let name = &ix.raw_method.sig.ident.to_string();
            let ix_name_camel =
                proc_macro2::Ident::new(&name.to_camel_case(), ix.raw_method.sig.ident.span());
            let raw_args: Vec<proc_macro2::TokenStream> = ix
                .args
                .iter()
                .map(|arg| {
                    format!("pub {}", parser::tts_to_string(&arg.raw_arg))
                        .parse()
                        .unwrap()
                })
                .collect();
            let ix_data_trait = {
                let sighash_arr = sighash(SIGHASH_GLOBAL_NAMESPACE, name);
                let sighash_tts: proc_macro2::TokenStream =
                    format!("{:?}", sighash_arr).parse().unwrap();
                quote! {
                    impl anchor_lang::InstructionData for #ix_name_camel {
                        fn data(&self) -> Vec<u8> {
                            let mut d = #sighash_tts.to_vec();
                            d.append(&mut self.try_to_vec().expect("Should always serialize"));
                            d
                        }
                    }
                }
            };
            // If no args, output a "unit" variant instead of a struct variant.
            if ix.args.is_empty() {
                quote! {
                    /// Instruction.
                    #[derive(AnchorSerialize, AnchorDeserialize)]
                    pub struct #ix_name_camel;

                    #ix_data_trait
                }
            } else {
                quote! {
                    /// Instruction.
                    #[derive(AnchorSerialize, AnchorDeserialize)]
                    pub struct #ix_name_camel {
                        #(#raw_args),*
                    }

                    #ix_data_trait
                }
            }
        })
        .collect();

    quote! {
        /// An Anchor generated module containing the program's set of
        /// instructions, where each method handler in the `#[program]` mod is
        /// associated with a struct defining the input arguments to the
        /// method. These should be used directly, when one wants to serialize
        /// Anchor instruction data, for example, when speciying
        /// instructions on a client.
        pub mod instruction {
            use super::*;

            /// Instruction struct definitions for `#[state]` methods.
            pub mod state {
                use super::*;

                #ctor_variant
                #(#state_method_variants)*
            }

            #(#variants)*
        }
    }
}
