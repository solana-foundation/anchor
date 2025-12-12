use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataEnum, DataStruct, DeriveInput, parse_macro_input};

#[proc_macro_derive(AbsolutePath)]
pub fn absolute_path_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let constructor = match input.data {
        syn::Data::Struct(data_struct) => derive_struct(data_struct),
        syn::Data::Enum(data_enum) => derive_enum(data_enum),
        syn::Data::Union(_) => {
            quote! { compile_error!("unions are not supported") }
        }
    };
    let name = input.ident;

    quote! {
        // Some fields/commands in the CLI are deprecated
        #[allow(deprecated)]
        impl AbsolutePath for #name {
            fn absolute(self) -> Self {
                #constructor
            }
        }
    }
    .into()
}

fn derive_struct(input: DataStruct) -> TokenStream {
    let members = input.fields.members();
    quote! {
        Self {
            #(#members: self.#members.absolute()),*
        }
    }
}

fn derive_enum(input: DataEnum) -> TokenStream {
    let variants = input.variants.into_iter().map(|variant| {
        let name = variant.ident;
        let members = variant.fields.members();
        let pattern = quote! {
            Self::#name {
                #(#members),*
            }
        };
        let members = variant.fields.members();
        let expression = quote! {
            Self::#name {
                #(#members: #members.absolute()),*
            }
        };
        quote! { #pattern => #expression }
    });
    quote! {
        match self {
            #(#variants),*
        }
    }
}
