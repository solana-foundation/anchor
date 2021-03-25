extern crate proc_macro;

use quote::quote;
use syn::parse_macro_input;

/// The `#[state]` attribute defines the program's state struct, i.e., the
/// program's global account singleton giving the program the illusion of state.
///
/// To allocate space into the account on initialization, pass in the account
/// size into the macro, e.g., `#[state(SIZE)]`. Otherwise, the size of the
/// account returned by the struct's `new` constructor will determine the
/// account size. When determining a size, make sure to reserve enough space
/// for the 8 byte account discriminator prepended to the account. That is,
/// always use 8 extra bytes.
#[proc_macro_attribute]
pub fn state(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item_struct = parse_macro_input!(input as syn::ItemStruct);
    let struct_ident = &item_struct.ident;

    let size_override = {
        if args.is_empty() {
            // No size override given. The account size is whatever is given
            // as the initialized value. Use the default implementation.
            quote! {
                impl anchor_lang::AccountSize for #struct_ident {
                    fn size(&self) -> std::result::Result<u64, anchor_lang::solana_program::program_error::ProgramError> {
                        Ok(8 + self
                           .try_to_vec()
                           .map_err(|_| ProgramError::Custom(1))?
                           .len() as u64)
                    }
                }
            }
        } else {
            let size = proc_macro2::TokenStream::from(args);
            // Size override given to the macro. Use it.
            quote! {
                impl anchor_lang::AccountSize for #struct_ident {
                    fn size(&self) -> std::result::Result<u64, anchor_lang::solana_program::program_error::ProgramError> {
                        Ok(#size)
                    }
                }
            }
        }
    };

    proc_macro::TokenStream::from(quote! {
        #[account]
        #item_struct

        #size_override
    })
}
