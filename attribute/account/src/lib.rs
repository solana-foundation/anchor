extern crate proc_macro;

use quote::quote;
use syn::parse_macro_input;

/// A data structure representing a Solana account.
#[proc_macro_attribute]
pub fn account(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let account_strct = parse_macro_input!(input as syn::ItemStruct);
    let account_name = &account_strct.ident;
    // Namespace the discriminator to prevent future collisions, e.g.,
    // if we (for some unforseen reason) wanted to hash other parts of the
    // program.
    let discriminator_preimage = format!("account:{}", account_name.to_string());

    let coder = quote! {
        impl anchor::AccountSerialize for #account_name {
            fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<(), ProgramError> {
                // TODO: we shouldn't have to hash at runtime. However, rust
                //       is not happy when trying to include solana-sdk from
                //       the proc-macro crate.
                let mut discriminator = [0u8; 8];
                discriminator.copy_from_slice(
                    &solana_program::hash::hash(
                        #discriminator_preimage.as_bytes(),
                    ).to_bytes()[..8],
                );

                writer.write_all(&discriminator).map_err(|_| ProgramError::InvalidAccountData)?;
                AnchorSerialize::serialize(
                    self,
                    writer
                )
                    .map_err(|_| ProgramError::InvalidAccountData)?;
                Ok(())
            }
        }

        impl anchor::AccountDeserialize for #account_name {
            fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError> {
                let mut discriminator = [0u8; 8];
                discriminator.copy_from_slice(
                    &solana_program::hash::hash(
                        #discriminator_preimage.as_bytes(),
                    ).to_bytes()[..8],
                );

                if buf.len() < discriminator.len() {
                    return Err(ProgramError::AccountDataTooSmall);
                }
                let given_disc = &buf[..8];
                if &discriminator != given_disc {
                    return Err(ProgramError::InvalidInstructionData);
                }
                Self::try_deserialize_unchecked(buf)
            }

            fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError> {
                let mut data: &[u8] = &buf[8..];
                AnchorDeserialize::deserialize(&mut data)
                    .map_err(|_| ProgramError::InvalidAccountData)
            }
        }
    };

    proc_macro::TokenStream::from(quote! {
        #[derive(AnchorSerialize, AnchorDeserialize, Clone)]
        #account_strct

        #coder
    })
}
