use crate::codegen::program::common::*;
use crate::parser;
use crate::Program;
use heck::CamelCase;
use quote::quote;

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
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
                    impl anchor_lang::Discriminator for #ix_name_camel {
                        const DISCRIMINATOR: [u8; 8] = #sighash_tts;
                    }
                    impl anchor_lang::InstructionData for #ix_name_camel {}
                    impl anchor_lang::Owner for #ix_name_camel {
                        fn owner() -> Pubkey {
                            ID
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


            #(#variants)*
        }
    }
}
