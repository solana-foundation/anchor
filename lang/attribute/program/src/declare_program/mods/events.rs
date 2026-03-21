use anchor_lang_idl::types::Idl;
use quote::{format_ident, quote};

use super::common::{convert_idl_type_def_to_ts, gen_discriminator};

pub fn gen_events_mod(idl: &Idl) -> syn::Result<proc_macro2::TokenStream> {
    let events = idl
        .events
        .iter()
        .map(|ev| {
            let name = format_ident!("{}", ev.name);
            let discriminator = gen_discriminator(&ev.discriminator);

            let ty_def = idl
                .types
                .iter()
                .find(|ty| ty.name == ev.name)
                .ok_or_else(|| {
                    syn::Error::new(
                        proc_macro2::Span::call_site(),
                        format!("Event type `{}` must exist in the IDL", ev.name),
                    )
                })?;
            let ty_def = convert_idl_type_def_to_ts(ty_def, &idl.types)?;

            Ok(quote! {
                #ty_def

                impl anchor_lang::Event for #name {
                    fn data(&self) -> Vec<u8> {
                        let mut data = Vec::with_capacity(256);
                        data.extend_from_slice(#name::DISCRIMINATOR);
                        self.serialize(&mut data)
                            .expect("event serialization should not fail");
                        data
                    }
                }

                impl anchor_lang::Discriminator for #name {
                    const DISCRIMINATOR: &'static [u8] = &#discriminator;
                }
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        /// Program event type definitions.
        pub mod events {
            use super::*;

            #(#events)*
        }
    })
}
