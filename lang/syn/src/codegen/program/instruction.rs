use crate::codegen::program::common::*;
use crate::parser;
use crate::Program;
use heck::CamelCase;
use quote::{quote, quote_spanned};
use syn::Type;

/// Returns true for primitives, common std types, and types wrapped in Option/Vec.
fn can_derive_common_trait(ty: &Type) -> bool {
    match ty {
        // Primitives - always support Clone/Debug
        Type::Path(path) if path.qself.is_none() => {
            let segments = &path.path.segments;
            if segments.is_empty() {
                return false;
            }
            // Use last segment to handle fully qualified paths like std::vec::Vec<T>
            let last_segment = segments.last().unwrap();
            let ident_str = last_segment.ident.to_string();

            // Check for primitives
            if matches!(
                ident_str.as_str(),
                "bool"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "isize"
                    | "u8"
                    | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "usize"
                    | "f32"
                    | "f64"
                    | "char"
                    | "str"
            ) {
                return true;
            }

            // For Option<T> and Vec<T>, check the inner type first
            if ident_str == "Option" || ident_str == "Vec" {
                if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return can_derive_common_trait(inner_ty);
                    }
                }
                // If we can't extract the inner type, Vec/Option themselves support Clone/Debug
                return true;
            }

            // Check for common std types that support Clone/Debug
            if matches!(ident_str.as_str(), "String" | "Pubkey") {
                return true;
            }

            // For user-defined types, we can't verify at macro time
            false
        }
        Type::Array(arr) => can_derive_common_trait(&arr.elem),
        Type::Tuple(tuple) => tuple.elems.iter().all(can_derive_common_trait),
        Type::Reference(reference) => can_derive_common_trait(&reference.elem),
        // For other types, be conservative
        _ => false,
    }
}

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

            // Check if all argument types can derive Clone and Debug
            // Note: all() returns true for empty iterators, so no need to check is_empty()
            let can_derive_traits = ix
                .args
                .iter()
                .all(|arg| can_derive_common_trait(&arg.raw_arg.ty));

            let traits_attr = if can_derive_traits {
                quote!(Clone, Debug,)
            } else {
                quote!()
            };

            let impls = {
                let discriminator = match ix.overrides.as_ref() {
                    Some(overrides) if overrides.discriminator.is_some() => {
                        overrides.discriminator.as_ref().unwrap().to_owned()
                    }
                    _ => gen_discriminator(SIGHASH_GLOBAL_NAMESPACE, name),
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
                    #[derive(AnchorSerialize, AnchorDeserialize, #traits_attr)]
                    pub struct #ix_name_camel;

                    #impls
                }
            } else {
                quote! {
                    #(#ix_cfgs)*
                    /// Instruction.
                    #[derive(AnchorSerialize, AnchorDeserialize, #traits_attr)]
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
        /// Anchor instruction data, for example, when specifying
        /// instructions on a client.
        pub mod instruction {
            use super::*;

            #(#variants)*
        }
    }
}
