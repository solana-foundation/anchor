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
///
/// # Zero Copy Deserialization
///
/// Similar to the `#[account]` attribute one can enable zero copy
/// deserialization by using the `zero_copy` argument:
///
/// ```ignore
/// #[state(zero_copy)]
/// ```
///
/// For more, see the [`account`](./attr.account.html) attribute.
#[deprecated(
    since = "0.14.0",
    note = "#[state] will be removed in a future version. Use a PDA with static seeds instead"
)]
#[proc_macro_attribute]
pub fn state(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item_struct = parse_macro_input!(input as syn::ItemStruct);
    let struct_ident = &item_struct.ident;
    let is_zero_copy = args.to_string() == "zero_copy";

    let size_override = {
        if args.is_empty() {
            // No size override given. The account size is whatever is given
            // as the initialized value. Use the default implementation.
            quote! {
                impl anchor_lang::__private::AccountSize for #struct_ident {
                    fn size(&self) -> std::result::Result<u64, anchor_lang::solana_program::program_error::ProgramError> {
                        Ok(8 + self
                           .try_to_vec()
                           .map_err(|_| anchor_lang::__private::ErrorCode::AccountDidNotSerialize)?
                           .len() as u64)
                    }
                }
            }
        } else if is_zero_copy {
            quote! {
                impl anchor_lang::__private::AccountSize for #struct_ident {
                    fn size(&self) -> std::result::Result<u64, anchor_lang::solana_program::program_error::ProgramError> {
                        let len = anchor_lang::__private::bytemuck::bytes_of(self).len() as u64;
                        Ok(8 + len)
                    }
                }
            }
        } else {
            let size = proc_macro2::TokenStream::from(args);
            // Size override given to the macro. Use it.
            quote! {
                impl anchor_lang::__private::AccountSize for #struct_ident {
                    fn size(&self) -> std::result::Result<u64, anchor_lang::solana_program::program_error::ProgramError> {
                        Ok(#size)
                    }
                }
            }
        }
    };

    let attribute = match is_zero_copy {
        false => quote! {
            #[cfg_attr(feature = "anchor-deprecated-state", account)]
            #[cfg_attr(not(feature = "anchor-deprecated-state"), account("state"))]
        },
        true => quote! {
            #[cfg_attr(feature = "anchor-deprecated-state", account(zero_copy))]
            #[cfg_attr(not(feature = "anchor-deprecated-state"), account("state", zero_copy))]
        },
    };

    proc_macro::TokenStream::from(quote! {
        #attribute
        #item_struct

        #size_override
    })
}
