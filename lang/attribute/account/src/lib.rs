extern crate proc_macro;

use quote::quote;
use syn::parse_macro_input;

/// A data structure representing a Solana account, implementing various traits:
///
/// - [`AccountSerialize`](./trait.AccountSerialize.html)
/// - [`AccountDeserialize`](./trait.AccountDeserialize.html)
/// - [`AnchorSerialize`](./trait.AnchorSerialize.html)
/// - [`AnchorDeserialize`](./trait.AnchorDeserialize.html)
///
/// When implementing account serialization traits the first 8 bytes are
/// reserved for a unique account discriminator, self described by the first 8
/// bytes of the SHA256 of the account's Rust ident.
///
/// As a result, any calls to `AccountDeserialize`'s `try_deserialize` will
/// check this discriminator. If it doesn't match, an invalid account was given,
/// and the account deserialization will exit with an error.
#[proc_macro_attribute]
pub fn account(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let namespace = args.to_string().replace("\"", "");

    let account_strct = parse_macro_input!(input as syn::ItemStruct);
    let account_name = &account_strct.ident;

    // Namespace the discriminator to prevent collisions.
    let discriminator_preimage = {
        if namespace == "" {
            format!("account:{}", account_name.to_string())
        } else {
            format!("{}:{}", namespace, account_name.to_string())
        }
    };

    let coder = quote! {
        impl anchor_lang::AccountSerialize for #account_name {
            fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> std::result::Result<(), ProgramError> {
                // TODO: we shouldn't have to hash at runtime. However, rust
                //       is not happy when trying to include solana-sdk from
                //       the proc-macro crate.
                let mut discriminator = [0u8; 8];
                discriminator.copy_from_slice(
                    &anchor_lang::solana_program::hash::hash(
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

        impl anchor_lang::AccountDeserialize for #account_name {

            fn try_deserialize(buf: &mut &[u8]) -> std::result::Result<Self, ProgramError> {
                let mut discriminator = [0u8; 8];
                discriminator.copy_from_slice(
                    &anchor_lang::solana_program::hash::hash(
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

            fn try_deserialize_unchecked(buf: &mut &[u8]) -> std::result::Result<Self, ProgramError> {
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
