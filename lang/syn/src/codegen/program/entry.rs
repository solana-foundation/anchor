use crate::Program;
use heck::CamelCase;
use quote::quote;

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    let program_id = match &program.program_id {
        Some(id) => quote! { #id },
        None => quote! { ID },
    };

    let name: proc_macro2::TokenStream = program.name.to_string().to_camel_case().parse().unwrap();

    quote! {
        #[cfg(not(feature = "no-entrypoint"))]
        anchor_lang::solana_program::entrypoint!(entry);
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

        pub mod program {
            use super::*;

            #[derive(Clone)]
            pub struct #name;
        }
    }
}
