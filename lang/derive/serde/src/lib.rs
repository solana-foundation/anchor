extern crate proc_macro;

#[cfg(feature = "lazy-account")]
mod lazy;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Fields, Ident, Item};

fn gen_borsh_serialize(input: TokenStream) -> TokenStream2 {
    let item: Item = syn::parse(input).unwrap();
    let res = match item {
        Item::Struct(item) => generate_struct_serialize(&item),
        Item::Enum(item) => generate_enum_serialize(&item),
        Item::Union(item) => generate_union_serialize(&item),
        // Derive macros can only be defined on structs, enums, and unions.
        _ => unreachable!(),
    };

    res
}

fn generate_struct_serialize(item: &syn::ItemStruct) -> TokenStream2 {
    let struct_name = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let serialize_fields = match &item.fields {
        Fields::Named(fields) => {
            let field_names = fields.named.iter().map(|f| &f.ident);
            quote! {
                #(
                    borsh::BorshSerialize::serialize(&self.#field_names, writer)?;
                )*
            }
        }
        Fields::Unnamed(fields) => {
            let indices = (0..fields.unnamed.len()).map(syn::Index::from);
            quote! {
                #(
                    borsh::BorshSerialize::serialize(&self.#indices, writer)?;
                )*
            }
        }
        Fields::Unit => quote! {},
    };

    quote! {
        impl #impl_generics borsh::BorshSerialize for #struct_name #ty_generics #where_clause {
            fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
                #serialize_fields
                Ok(())
            }
        }
    }
}

fn generate_enum_serialize(item: &syn::ItemEnum) -> TokenStream2 {
    let enum_name = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let serialize_variants = item.variants.iter().enumerate().map(|(idx, variant)| {
        let variant_name = &variant.ident;
        let idx_u8 = idx as u8;

        match &variant.fields {
            Fields::Named(fields) => {
                let field_names: Vec<_> = fields
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();
                quote! {
                    #enum_name::#variant_name { #(#field_names),* } => {
                        writer.write_all(&[#idx_u8])?;
                        #(
                            borsh::BorshSerialize::serialize(#field_names, writer)?;
                        )*
                    }
                }
            }
            Fields::Unnamed(fields) => {
                let field_names: Vec<_> = (0..fields.unnamed.len())
                    .map(|i| Ident::new(&format!("field{}", i), Span::call_site()))
                    .collect();
                quote! {
                    #enum_name::#variant_name(#(#field_names),*) => {
                        writer.write_all(&[#idx_u8])?;
                        #(
                            borsh::BorshSerialize::serialize(#field_names, writer)?;
                        )*
                    }
                }
            }
            Fields::Unit => {
                quote! {
                    #enum_name::#variant_name => {
                        writer.write_all(&[#idx_u8])?;
                    }
                }
            }
        }
    });

    quote! {
        impl #impl_generics borsh::BorshSerialize for #enum_name #ty_generics #where_clause {
            fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
                match self {
                    #(#serialize_variants)*
                }
                Ok(())
            }
        }
    }
}

fn generate_union_serialize(item: &syn::ItemUnion) -> TokenStream2 {
    syn::Error::new_spanned(item, "Unions are not supported by borsh").to_compile_error()
}

#[proc_macro_derive(AnchorSerialize, attributes(borsh_skip))]
pub fn anchor_serialize(input: TokenStream) -> TokenStream {
    #[cfg(not(feature = "idl-build"))]
    let ret = gen_borsh_serialize(input);
    #[cfg(feature = "idl-build")]
    let ret = gen_borsh_serialize(input.clone());

    #[cfg(feature = "idl-build")]
    {
        use anchor_syn::idl::*;
        use quote::quote;

        let idl_build_impl = match syn::parse(input).unwrap() {
            Item::Struct(item) => impl_idl_build_struct(&item),
            Item::Enum(item) => impl_idl_build_enum(&item),
            Item::Union(item) => impl_idl_build_union(&item),
            // Derive macros can only be defined on structs, enums, and unions.
            _ => unreachable!(),
        };

        return TokenStream::from(quote! {
            #ret
            #idl_build_impl
        });
    };

    #[allow(unreachable_code)]
    TokenStream::from(ret)
}

fn gen_borsh_deserialize(input: TokenStream) -> TokenStream2 {
    let item: Item = syn::parse(input).unwrap();
    let res = match item {
        Item::Struct(item) => generate_struct_deserialize(&item),
        Item::Enum(item) => generate_enum_deserialize(&item),
        Item::Union(item) => generate_union_deserialize(&item),
        // Derive macros can only be defined on structs, enums, and unions.
        _ => unreachable!(),
    };

    res
}

fn generate_struct_deserialize(item: &syn::ItemStruct) -> TokenStream2 {
    let struct_name = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let deserialize_fields = match &item.fields {
        Fields::Named(fields) => {
            let field_names: Vec<_> = fields
                .named
                .iter()
                .map(|f| f.ident.as_ref().unwrap())
                .collect();
            quote! {
                Ok(Self {
                    #(
                        #field_names: borsh::BorshDeserialize::deserialize_reader(reader)?,
                    )*
                })
            }
        }
        Fields::Unnamed(fields) => {
            let field_deserializations = (0..fields.unnamed.len()).map(|_| {
                quote! { borsh::BorshDeserialize::deserialize_reader(reader)? }
            });
            quote! {
                Ok(Self(
                    #(#field_deserializations),*
                ))
            }
        }
        Fields::Unit => {
            quote! {
                Ok(Self)
            }
        }
    };

    quote! {
        impl #impl_generics borsh::BorshDeserialize for #struct_name #ty_generics #where_clause {
            fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
                #deserialize_fields
            }
        }
    }
}

fn generate_enum_deserialize(item: &syn::ItemEnum) -> TokenStream2 {
    let enum_name = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let deserialize_variants = item.variants.iter().enumerate().map(|(idx, variant)| {
        let variant_name = &variant.ident;
        let idx_u8 = idx as u8;

        let construct = match &variant.fields {
            Fields::Named(fields) => {
                let field_names: Vec<_> = fields
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();
                quote! {
                    #enum_name::#variant_name {
                        #(
                            #field_names: borsh::BorshDeserialize::deserialize_reader(reader)?,
                        )*
                    }
                }
            }
            Fields::Unnamed(fields) => {
                let field_deserializations = (0..fields.unnamed.len()).map(|_| {
                    quote! { borsh::BorshDeserialize::deserialize_reader(reader)? }
                });
                quote! {
                    #enum_name::#variant_name(
                        #(#field_deserializations),*
                    )
                }
            }
            Fields::Unit => {
                quote! {
                    #enum_name::#variant_name
                }
            }
        };

        quote! {
            #idx_u8 => Ok(#construct),
        }
    });

    quote! {
        impl #impl_generics borsh::BorshDeserialize for #enum_name #ty_generics #where_clause {
            fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
                let mut variant_idx = [0u8; 1];
                reader.read_exact(&mut variant_idx)?;
                match variant_idx[0] {
                    #(#deserialize_variants)*
                    _ => Err(borsh::io::Error::new(
                        borsh::io::ErrorKind::InvalidData,
                        format!("Invalid enum variant index: {}", variant_idx[0]),
                    )),
                }
            }
        }
    }
}

fn generate_union_deserialize(item: &syn::ItemUnion) -> TokenStream2 {
    syn::Error::new_spanned(item, "Unions are not supported by borsh").to_compile_error()
}

#[proc_macro_derive(AnchorDeserialize, attributes(borsh_skip, borsh_init))]
pub fn borsh_deserialize(input: TokenStream) -> TokenStream {
    #[cfg(feature = "lazy-account")]
    {
        let deser = gen_borsh_deserialize(input.clone());
        let lazy = lazy::gen_lazy(input).unwrap_or_else(|e| e.to_compile_error());
        quote::quote! {
            #deser
            #lazy
        }
        .into()
    }
    #[cfg(not(feature = "lazy-account"))]
    gen_borsh_deserialize(input).into()
}

#[cfg(feature = "lazy-account")]
#[proc_macro_derive(Lazy)]
pub fn lazy(input: TokenStream) -> TokenStream {
    lazy::gen_lazy(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
