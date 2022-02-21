use crate::program_codegen::dispatch;
use crate::Program;
use heck::CamelCase;
use quote::quote;

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    let name: proc_macro2::TokenStream = program.name.to_string().to_camel_case().parse().unwrap();
    let fallback_maybe = dispatch::gen_fallback(program).unwrap_or(quote! {
        Err(anchor_lang::error::ErrorCode::InstructionMissing.into())
    });
    quote! {
        #[cfg(not(feature = "no-entrypoint"))]
        anchor_lang::solana_program::entrypoint!(entry);
        /// The Anchor codegen exposes a programming model where a user defines
        /// a set of methods inside of a `#[program]` module in a way similar
        /// to writing RPC request handlers. The macro then generates a bunch of
        /// code wrapping these user defined methods into something that can be
        /// executed on Solana.
        ///
        /// These methods fall into one of three categories, each of which
        /// can be considered a different "namespace" of the program.
        ///
        /// 1) Global methods - regular methods inside of the `#[program]`.
        /// 2) State methods - associated methods inside a `#[state]` struct.
        /// 3) Interface methods - methods inside a strait struct's
        ///    implementation of an `#[interface]` trait.
        ///
        /// Care must be taken by the codegen to prevent collisions between
        /// methods in these different namespaces. For this reason, Anchor uses
        /// a variant of sighash to perform method dispatch, rather than
        /// something like a simple enum variant discriminator.
        ///
        /// The execution flow of the generated code can be roughly outlined:
        ///
        /// * Start program via the entrypoint.
        /// * Strip method identifier off the first 8 bytes of the instruction
        ///   data and invoke the identified method. The method identifier
        ///   is a variant of sighash. See docs.rs for `anchor_lang` for details.
        /// * If the method identifier is an IDL identifier, execute the IDL
        ///   instructions, which are a special set of hardcoded instructions
        ///   baked into every Anchor program. Then exit.
        /// * Otherwise, the method identifier is for a user defined
        ///   instruction, i.e., one of the methods in the user defined
        ///   `#[program]` module. Perform method dispatch, i.e., execute the
        ///   big match statement mapping method identifier to method handler
        ///   wrapper.
        /// * Run the method handler wrapper. This wraps the code the user
        ///   actually wrote, deserializing the accounts, constructing the
        ///   context, invoking the user's code, and finally running the exit
        ///   routine, which typically persists account changes.
        ///
        /// The `entry` function here, defines the standard entry to a Solana
        /// program, where execution begins.
        pub fn entry(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> anchor_lang::solana_program::entrypoint::ProgramResult {
            try_entry(program_id, accounts, data).map_err(|e| {
                e.log();
                e.into()
            })
        }

        fn try_entry(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> anchor_lang::Result<()> {
            #[cfg(feature = "anchor-debug")]
            {
                msg!("anchor-debug is active");
            }
            if *program_id != ID {
                return Err(anchor_lang::error::ErrorCode::DeclaredProgramIdMismatch.into());
            }
            if data.len() < 8 {
                return #fallback_maybe;
            }

            dispatch(program_id, accounts, data)
        }

        /// Module representing the program.
        pub mod program {
            use super::*;

            /// Type representing the program.
            #[derive(Clone)]
            pub struct #name;

            impl anchor_lang::Id for #name {
                fn id() -> Pubkey {
                    ID
                }
            }
        }
    }
}
