extern crate proc_macro;

use quote::quote;
use syn::parse_macro_input;

/// The event attribute allows a struct to be used with
/// [emit!](./macro.emit.html) so that programs can log significant events in
/// their programs that clients can subscribe to. Currently, this macro is for
/// structs only.
///
/// See the [`emit!` macro](emit!) for an example.
#[proc_macro_attribute]
pub fn event(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let event_strct = parse_macro_input!(input as syn::ItemStruct);

    let event_name = &event_strct.ident;

    let discriminator: proc_macro2::TokenStream = {
        let discriminator_preimage = format!("event:{}", event_name);
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(
            &anchor_syn::hash::hash(discriminator_preimage.as_bytes()).to_bytes()[..8],
        );
        format!("{:?}", discriminator).parse().unwrap()
    };

    proc_macro::TokenStream::from(quote! {
        #[derive(anchor_lang::__private::EventIndex, AnchorSerialize, AnchorDeserialize)]
        #event_strct

        impl anchor_lang::Event for #event_name {
            fn data(&self) -> Vec<u8> {
                let mut d = #discriminator.to_vec();
                d.append(&mut self.try_to_vec().unwrap());
                d
            }
        }

        impl anchor_lang::Discriminator for #event_name {
            fn discriminator() -> [u8; 8] {
                #discriminator
            }
        }
    })
}

/// Logs an event that can be subscribed to by clients.
/// Uses the [`sol_log_data`](https://docs.rs/solana-program/latest/solana_program/log/fn.sol_log_data.html)
/// syscall which results in the following log:
/// ```ignore
/// Program data: <Base64EncodedEvent>
/// ```
/// # Example
///
/// ```rust,ignore
/// use anchor_lang::prelude::*;
///
/// // handler function inside #[program]
/// pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
///     emit!(MyEvent {
///         data: 5,
///         label: [1,2,3,4,5],
///     });
///     Ok(())
/// }
///
/// #[event]
/// pub struct MyEvent {
///     pub data: u64,
///     pub label: [u8; 5],
/// }
/// ```
#[proc_macro]
pub fn emit(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let data: proc_macro2::TokenStream = input.into();
    proc_macro::TokenStream::from(quote! {
        {
            anchor_lang::solana_program::log::sol_log_data(&[&anchor_lang::Event::data(&#data)]);
        }
    })
}

// EventIndex is a marker macro. It functionally does nothing other than
// allow one to mark fields with the `#[index]` inert attribute, which is
// used to add metadata to IDLs.
#[proc_macro_derive(EventIndex, attributes(index))]
pub fn derive_event(_item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    proc_macro::TokenStream::from(quote! {})
}
