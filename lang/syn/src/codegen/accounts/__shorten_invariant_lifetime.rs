use {
    super::{generics, ParsedGenerics},
    crate::AccountsStruct,
    quote::quote,
};

pub fn generate(accs: &AccountsStruct) -> proc_macro2::TokenStream {
    let name = &accs.ident;
    let ParsedGenerics {
        combined_generics,
        trait_generics: _,
        struct_generics,
        where_clause,
    } = generics(accs);

    let shorten_invariant_lifetime = if accs.generics.lt_token.is_some() {
        quote! {
            unsafe fn __shorten_invariant_lifetime<'__a, '__info: '__a>(
                value: &'__a mut #name<'__info>,
            ) -> &'__a mut #name<'__a> {
                unsafe { ::core::mem::transmute(value) }
            }
        }
    } else {
        quote! {
            fn __shorten_invariant_lifetime(value: &mut Self) -> &mut Self {
                value
            }
        }
    };

    quote! {
        #[automatically_derived]
        impl<#combined_generics> #name<#struct_generics> #where_clause {
            #[doc(hidden)]
            #[inline(always)]
            #shorten_invariant_lifetime
        }
    }
}
