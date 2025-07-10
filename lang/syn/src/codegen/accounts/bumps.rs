use crate::{
    codegen::accounts::{generics, ParsedGenerics},
    *,
};
use std::fmt::Display;

use super::constraints;

pub fn generate_bumps_name<T: Display>(anchor_ident: &T) -> Ident {
    Ident::new(&format!("{}Bumps", anchor_ident), Span::call_site())
}

pub fn generate(accs: &AccountsStruct) -> proc_macro2::TokenStream {
    let name = &accs.ident;
    let bumps_name = generate_bumps_name(name);
    let ParsedGenerics {
        combined_generics,
        trait_generics: _,
        struct_generics,
        where_clause,
    } = generics(accs);

    let (bump_fields, bump_default_fields): (
        Vec<proc_macro2::TokenStream>,
        Vec<proc_macro2::TokenStream>,
    ) = accs
        .fields
        .iter()
        .filter_map(|af| {
            let ident = af.ident();

            match af {
                AccountField::Field(f) => {
                    let constraints = constraints::linearize(&f.constraints);
                    let (bump_field, bump_default_field) = if f.is_optional {
                        (quote!(pub #ident: Option<u8>), quote!(#ident: None))
                    } else {
                        (quote!(pub #ident: u8), quote!(#ident: u8::MAX))
                    };

                    for c in constraints.iter() {
                        // Verify this in super::constraints
                        // The bump is only cached if
                        // - PDA is marked as init
                        // - PDA is not init, but marked with bump without a target

                        match c {
                            Constraint::Seeds(c) => {
                                if !c.is_init && c.bump.is_none() {
                                    return Some((bump_field, bump_default_field));
                                }
                            }
                            Constraint::Init(c) => {
                                if c.seeds.is_some() {
                                    return Some((bump_field, bump_default_field));
                                }
                            }
                            _ => (),
                        }
                    }
                    None
                }
                AccountField::CompositeField(s) => {
                    let comp_bumps_struct = generate_bumps_name(&s.symbol);
                    let bumps = quote!(pub #ident: #comp_bumps_struct);
                    let bumps_default = quote!(#ident: #comp_bumps_struct::default());

                    Some((bumps, bumps_default))
                }
            }
        })
        .unzip();

    quote! {
        #[derive(Debug)]
        pub struct #bumps_name {
            #(#bump_fields,)*
            /// Stack of shard indices encoded as little-endian bytes. Fixed-size to avoid heap allocation.
            __shard_index_stack: [[u8; 8]; 8],
            __shard_depth: u8,
        }

        impl Default for #bumps_name {
            fn default() -> Self {
                #bumps_name {
                    #(#bump_default_fields,)*
                    __shard_index_stack: [[0u8; 8]; 8],
                    __shard_depth: 0,
                }
            }
        }

        impl #bumps_name {
            /// Pushes the given shard index (little-endian) onto the seed stack.
            pub fn __push_shard_index(&mut self, idx: u64) {
                let depth = self.__shard_depth as usize;
                if depth < self.__shard_index_stack.len() {
                    self.__shard_index_stack[depth] = idx.to_le_bytes();
                    self.__shard_depth += 1;
                }
            }

            /// Removes the most recently pushed shard index.
            pub fn __pop_shard_index(&mut self) {
                if self.__shard_depth > 0 {
                    self.__shard_depth -= 1;
                }
            }

            /// Returns the seed bytes for the current shard index, if any.
            pub fn __current_shard_index_seed(&self) -> Option<&[u8]> {
                if self.__shard_depth == 0 {
                    None
                } else {
                    Some(&self.__shard_index_stack[(self.__shard_depth - 1) as usize][..])
                }
            }
        }

        impl<#combined_generics> anchor_lang::Bumps for #name<#struct_generics> #where_clause {
            type Bumps = #bumps_name;
        }
    }
}
