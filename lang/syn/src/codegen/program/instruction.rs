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
            let ix_cfgs = &ix.cfgs;
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
            let impls = {
                let discriminator = match ix.overrides.as_ref() {
                    Some(overrides) if overrides.discriminator.is_some() => {
                        overrides.discriminator.as_ref().unwrap().to_owned()
                    }
                    // TODO: Remove `interface_discriminator`
                    _ => match &ix.interface_discriminator {
                        Some(disc) => format!("&{disc:?}").parse().unwrap(),
                        _ => gen_discriminator(SIGHASH_GLOBAL_NAMESPACE, name),
                    },
                };

                quote! {
                    #(#ix_cfgs)*
                    impl anchor_lang::Discriminator for #ix_name_camel {
                        const DISCRIMINATOR: &'static [u8] = #discriminator;
                    }
                    #(#ix_cfgs)*
                    impl anchor_lang::InstructionData for #ix_name_camel {}
                    #(#ix_cfgs)*
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
                    #(#ix_cfgs)*
                    /// Instruction.
                    #[derive(AnchorSerialize, AnchorDeserialize)]
                    pub struct #ix_name_camel;

                    #impls
                }
            } else {
                quote! {
                    #(#ix_cfgs)*
                    /// Instruction.
                    #[derive(AnchorSerialize, AnchorDeserialize)]
                    pub struct #ix_name_camel {
                        #(#raw_args),*
                    }

                    #impls
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
