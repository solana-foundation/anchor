use crate::Program;
use heck::CamelCase;
use quote::quote;

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    let name: proc_macro2::TokenStream = program.name.to_string().to_camel_case().parse().unwrap();
    quote! {
        #[cfg(not(feature = "no-entrypoint"))]
        anchor_lang::pinocchio_runtime::entrypoint::entrypoint!(entry);
        /// The Anchor codegen exposes a programming model where a user defines
        /// a set of methods inside of a `#[program]` module in a way similar
        /// to writing RPC request handlers. The macro then generates a bunch of
        /// code wrapping these user defined methods into something that can be
        /// executed on Solana.
        ///
        /// These methods fall into one category for now.
        ///
        /// Global methods - regular methods inside of the `#[program]`.
        ///
        /// Care must be taken by the codegen to prevent collisions between
        /// methods in these different namespaces. For this reason, Anchor uses
        /// a variant of sighash to perform method dispatch, rather than
        /// something like a simple enum variant discriminator.
        ///
        /// The execution flow of the generated code can be roughly outlined:
        ///
        /// * Start program via the entrypoint.
        /// * Check whether the declared program id matches the input program
        ///   id. If it's not, return an error.
        /// * Find and invoke the method based on whether the instruction data
        ///   starts with the method's discriminator.
        /// * Run the method handler wrapper. This wraps the code the user
        ///   actually wrote, deserializing the accounts, constructing the
        ///   context, invoking the user's code, and finally running the exit
        ///   routine, which typically persists account changes.
        ///
        /// The `entry` function here, defines the standard entry to a Solana
        /// program, where execution begins.
        /// Pinocchio's entrypoint passes program_id as &[u8; 32] and Pinocchio's AccountInfo.
        /// Pinocchio's AccountInfo is compatible with Solana's runtime at the binary level,
        /// but the Rust types are different. We need to accept Pinocchio's AccountInfo and
        /// convert it to Solana's AccountInfo format that Anchor expects.
        ///
        /// Note: Pinocchio's entrypoint provides accounts in a zero-copy format using raw pointers,
        /// while Solana's AccountInfo uses RefCell for interior mutability. The conversion needs
        /// to preserve is_signer, is_writable, and other account metadata.
        pub fn entry<'info>(
            program_id: &anchor_lang::pinocchio_runtime::pubkey::PinocchioPubkey,
            accounts: &'info [AccountInfo],
            data: &[u8]
        ) -> anchor_lang::pinocchio_runtime::entrypoint::ProgramResult {
            // Convert Pinocchio's Pubkey ([u8; 32]) to Solana's Pubkey
            let program_id_pubkey = Pubkey::from(*program_id);

            // Pinocchio's AccountInfo is now used directly throughout Anchor
            // No conversion needed - Pinocchio's AccountInfo is compatible with Anchor's runtime
            try_entry(&program_id_pubkey, accounts, data).map_err(|e| {
                e.log();
                e.into()
            })
        }

        fn try_entry<'info>(program_id: &Pubkey, accounts: &'info [AccountInfo], data: &[u8]) -> anchor_lang::Result<()> {
            #[cfg(feature = "anchor-debug")]
            {
                msg!("anchor-debug is active");
            }
            if *program_id != ID {
                return Err(anchor_lang::error::ErrorCode::DeclaredProgramIdMismatch.into());
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
