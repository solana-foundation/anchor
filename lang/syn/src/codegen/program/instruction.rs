use crate::codegen::program::common::*;
use crate::parser;
use crate::Program;
use heck::CamelCase;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    let variants: Vec<proc_macro2::TokenStream> = program
        .ixs
        .iter()
        .map(|ix| {
            let name = &ix.raw_method.sig.ident.to_string();
            let ix_cfgs = &ix.cfgs;
            let Ok(ix_name_camel) = syn::parse_str::<syn::Ident>(&name.to_camel_case()) else {
                return quote_spanned! { ix.raw_method.sig.ident.span()=>
                    compile_error!("failed to parse ix method name after conversion to camelCase");
                };
            };
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
                let ix_span = ix.raw_method.span();
                let spanned_name = quote_spanned! { ix_span => #ix_name_camel };

                quote! {
                    #(#ix_cfgs)*
                    impl anchor_lang::Discriminator for #spanned_name {
                        const DISCRIMINATOR: &'static [u8] = #discriminator;
                    }
                    #(#ix_cfgs)*
                    impl anchor_lang::InstructionData for #spanned_name {}
                    #(#ix_cfgs)*
                    impl anchor_lang::Owner for #spanned_name {
                        fn owner() -> Pubkey {
                            ID
                        }
                    }
                }
            };
            // If no args, output a "unit" variant instead of a struct variant.
            let ix_span = ix.raw_method.span();
            let spanned_name = quote_spanned! { ix_span => #ix_name_camel };
            if ix.args.is_empty() {
                quote! {
                    #(#ix_cfgs)*
                    /// Instruction.
                    #[derive(AnchorSerialize, AnchorDeserialize)]
                    pub struct #spanned_name;

                    #impls
                }
            } else {
                quote! {
                    #(#ix_cfgs)*
                    /// Instruction.
                    #[derive(AnchorSerialize, AnchorDeserialize)]
                    pub struct #spanned_name {
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
        /// Anchor instruction data, for example, when specifying
        /// instructions on a client.
        pub mod instruction {
            use super::*;

            #(#variants)*
        }
    }
}
