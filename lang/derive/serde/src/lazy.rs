use proc_macro2::Literal;
use quote::{format_ident, quote};
use syn::parse_quote;
use syn::{spanned::Spanned, Fields, GenericParam, Item};

pub fn gen_lazy(input: proc_macro::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let item = syn::parse::<Item>(input)?;
    let (name, generics, size, sized) = match &item {
        Item::Struct(strct) => (
            &strct.ident,
            &strct.generics,
            sum_fields(&strct.fields),
            strct
                .fields
                .iter()
                .map(|field| &field.ty)
                .map(|ty| quote! { <#ty as anchor_lang::__private::Lazy>::SIZED })
                .fold(quote!(true), |acc, sized| quote! { #acc && #sized }),
        ),
        Item::Enum(enm) => {
            let arms = enm
                .variants
                .iter()
                .map(|variant| sum_fields(&variant.fields))
                .enumerate()
                .map(|(i, size)| (Literal::usize_unsuffixed(i), size))
                .map(|(i, size)| quote! { Some(#i) => { #size } });

            (
                &enm.ident,
                &enm.generics,
                quote! {
                    1 + match buf.first() {
                        #(#arms,)*
                        _ => unreachable!(),
                    }
                },
                quote!(false),
            )
        }
        Item::Union(_) => return Err(syn::Error::new(item.span(), "Unions are not supported")),
        _ => unreachable!(),
    };

    let mut impl_generics_with_bounds = generics.clone();
    let where_clause = impl_generics_with_bounds.make_where_clause();
    for param in &generics.params {
        if let GenericParam::Type(ty) = param {
            let ty = &ty.ident;
            where_clause
                .predicates
                .push(parse_quote!(#ty: anchor_lang::__private::Lazy));
        }
    }
    let (impl_generics, ty_generics, where_clause) = impl_generics_with_bounds.split_for_impl();

    Ok(quote! {
        impl #impl_generics anchor_lang::__private::Lazy for #name #ty_generics #where_clause {
            const SIZED: bool = #sized;

            #[inline(always)]
            fn size_of(buf: &[u8]) -> usize {
                #size
            }
        }
    })
}

fn sum_fields(fields: &Fields) -> proc_macro2::TokenStream {
    let names = fields
        .iter()
        .enumerate()
        .map(|(i, _)| format_ident!("s{i}"))
        .collect::<Vec<_>>();
    let declarations = fields.iter().enumerate().map(|(i, field)| {
        let ty = &field.ty;
        let name = &names[i];
        let sum = &names[..i];
        let buf = quote! { &buf[0 #(+ #sum)*..] };
        quote! { let #name = <#ty as anchor_lang::__private::Lazy>::size_of(#buf) }
    });

    quote! {
       #(#declarations;)*
       0 #(+ #names)*
    }
}
