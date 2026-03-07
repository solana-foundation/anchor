use crate::Program;
use heck::CamelCase;
use quote::quote;

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    let program_id = match &program.program_id {
        Some(id) => quote! { #id },
        None => quote! { ID },
    };

    let maybe_id_const = match &program.program_id {
        Some(id) => quote! {
            pub const ID: anchor_lang::solana_program::pubkey::Pubkey = #id;
        },
        None => quote! {},
    };

    let name: proc_macro2::TokenStream = program.name.to_string().to_camel_case().parse().unwrap();

    quote! {
        #maybe_id_const
        #[cfg(not(feature = "no-entrypoint"))]
        anchor_lang::solana_program::entrypoint!(entry);
        /// The Anchor codegen exposes a programming model where a user defines
        /// a set of methods inside of a `#[program]` module in a way similar
        /// to writing RPC request handlers. The macro then generates a bunch of
        /// code wrapping these user defined methods into something that can be
        /// executed on Solana.
        ///
        /// The execution flow of the generated code can be roughly outlined:
        ///
        /// * Start program via the entrypoint.
        /// * Check whether the program id matches `AnchorProgram::ID`. If not, return an error.
        /// * Find and invoke the method based on whether the instruction data
        ///   starts with the method's discriminator.
        /// * Run the method handler wrapper. This wraps the code the user
        ///   actually wrote, deserializing the accounts, constructing the
        ///   context, invoking the user's code, and finally running the exit
        ///   routine, which typically persists account changes.
        pub fn entry<'info>(
            program_id: &Pubkey,
            accounts: &'info [AccountInfo<'info>],
            data: &[u8],
        ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
            <program::#name as anchor_lang::AnchorProgram>::entrypoint(program_id, accounts, data)
        }

        impl anchor_lang::AnchorProgram for program::#name {
            const ID: Pubkey = #program_id;

            #[cfg(feature = "anchor-debug")]
            fn entrypoint<'info>(
                program_id: &Pubkey,
                accounts: &'info [AccountInfo<'info>],
                data: &[u8],
            ) -> std::result::Result<(), anchor_lang::solana_program::program_error::ProgramError> {
                anchor_lang::prelude::msg!("anchor-debug is active");
                if *program_id != Self::ID {
                    return Err(anchor_lang::error::ErrorCode::DeclaredProgramIdMismatch.into())
                        .map_err(Self::handle_error);
                }
                Self::dispatch(program_id, accounts, data).map_err(Self::handle_error)
            }

            fn dispatch<'info>(
                program_id: &Pubkey,
                accounts: &'info [AccountInfo<'info>],
                data: &[u8],
            ) -> anchor_lang::Result<()> {
                dispatch(program_id, accounts, data)
            }
        }

        /// Module representing the program.
        pub mod program {
            use super::*;

            /// Type representing the program.
            #[derive(Clone)]
            pub struct #name;
        }
    }
}
