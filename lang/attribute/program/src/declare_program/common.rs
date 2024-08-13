use anchor_lang_idl::types::{
    Idl, IdlArrayLen, IdlDefinedFields, IdlField, IdlGenericArg, IdlRepr, IdlSerialization,
    IdlType, IdlTypeDef, IdlTypeDefGeneric, IdlTypeDefTy,
};
use proc_macro2::Literal;
use quote::{format_ident, quote};

/// This function should ideally return the absolute path to the declared program's id but because
/// `proc_macro2::Span::call_site().source_file().path()` is behind an unstable feature flag, we
/// are not able to reliably decide where the definition is.
pub fn get_canonical_program_id() -> proc_macro2::TokenStream {
    quote! { super::__ID }
}

pub fn gen_docs(docs: &[String]) -> proc_macro2::TokenStream {
    let docs = docs
        .iter()
        .map(|doc| format!("{}{doc}", if doc.is_empty() { "" } else { " " }))
        .map(|doc| quote! { #[doc = #doc] });
    quote! { #(#docs)* }
}

pub fn gen_discriminator(disc: &[u8]) -> proc_macro2::TokenStream {
    quote! { [#(#disc), *] }
}

pub fn gen_accounts_common(idl: &Idl, prefix: &str) -> proc_macro2::TokenStream {
    let re_exports = idl
        .instructions
        .iter()
        .map(|ix| format_ident!("__{}_accounts_{}", prefix, ix.name))
        .map(|ident| quote! { pub use super::internal::#ident::*; });

    quote! {
        pub mod accounts {
            #(#re_exports)*
        }
    }
}

pub fn convert_idl_type_to_syn_type(ty: &IdlType) -> syn::Type {
    syn::parse_str(&convert_idl_type_to_str(ty)).unwrap()
}

// TODO: Impl `ToString` for `IdlType`
pub fn convert_idl_type_to_str(ty: &IdlType) -> String {
    match ty {
        IdlType::Bool => "bool".into(),
        IdlType::U8 => "u8".into(),
        IdlType::I8 => "i8".into(),
        IdlType::U16 => "u16".into(),
        IdlType::I16 => "i16".into(),
        IdlType::U32 => "u32".into(),
        IdlType::I32 => "i32".into(),
        IdlType::F32 => "f32".into(),
        IdlType::U64 => "u64".into(),
        IdlType::I64 => "i64".into(),
        IdlType::F64 => "f64".into(),
        IdlType::U128 => "u128".into(),
        IdlType::I128 => "i128".into(),
        IdlType::U256 => "u256".into(),
        IdlType::I256 => "i256".into(),
        IdlType::Bytes => "Vec<u8>".into(),
        IdlType::String => "String".into(),
        IdlType::Pubkey => "Pubkey".into(),
        IdlType::Option(ty) => format!("Option<{}>", convert_idl_type_to_str(ty)),
        IdlType::Vec(ty) => format!("Vec<{}>", convert_idl_type_to_str(ty)),
        IdlType::Array(ty, len) => format!(
            "[{}; {}]",
            convert_idl_type_to_str(ty),
            match len {
                IdlArrayLen::Generic(len) => len.into(),
                IdlArrayLen::Value(len) => len.to_string(),
            }
        ),
        IdlType::Defined { name, generics } => generics
            .iter()
            .map(|generic| match generic {
                IdlGenericArg::Type { ty } => convert_idl_type_to_str(ty),
                IdlGenericArg::Const { value } => value.into(),
            })
            .reduce(|mut acc, cur| {
                if !acc.is_empty() {
                    acc.push(',');
                }
                acc.push_str(&cur);
                acc
            })
            .map(|generics| format!("{name}<{generics}>"))
            .unwrap_or(name.into()),
        IdlType::Generic(ty) => ty.into(),
        _ => unimplemented!("{ty:?}"),
    }
}

pub fn convert_idl_type_def_to_ts(
    ty_def: &IdlTypeDef,
    ty_defs: &[IdlTypeDef],
) -> proc_macro2::TokenStream {
    let name = format_ident!("{}", ty_def.name);
    let docs = gen_docs(&ty_def.docs);

    let generics = {
        let generics = ty_def
            .generics
            .iter()
            .map(|generic| match generic {
                IdlTypeDefGeneric::Type { name } => {
                    let name = format_ident!("{}", name);
                    quote! { #name }
                }
                IdlTypeDefGeneric::Const { name, ty } => {
                    let name = format_ident!("{}", name);
                    let ty = format_ident!("{}", ty);
                    quote! { const #name: #ty }
                }
            })
            .collect::<Vec<_>>();
        if generics.is_empty() {
            quote!()
        } else {
            quote!(<#(#generics,)*>)
        }
    };

    let attrs = {
        let debug_attr = quote!(#[derive(Debug)]);

        let default_attr = can_derive_default(ty_def, ty_defs)
            .then(|| quote!(#[derive(Default)]))
            .unwrap_or_default();

        let ser_attr = match &ty_def.serialization {
            IdlSerialization::Borsh => quote!(#[derive(AnchorSerialize, AnchorDeserialize)]),
            IdlSerialization::Bytemuck => quote!(#[zero_copy]),
            IdlSerialization::BytemuckUnsafe => quote!(#[zero_copy(unsafe)]),
            _ => unimplemented!("{:?}", ty_def.serialization),
        };

        let clone_attr = matches!(ty_def.serialization, IdlSerialization::Borsh)
            .then(|| quote!(#[derive(Clone)]))
            .unwrap_or_default();

        let copy_attr = matches!(ty_def.serialization, IdlSerialization::Borsh)
            .then(|| can_derive_copy(ty_def, ty_defs).then(|| quote!(#[derive(Copy)])))
            .flatten()
            .unwrap_or_default();

        quote! {
            #debug_attr
            #default_attr
            #ser_attr
            #clone_attr
            #copy_attr
        }
    };

    let repr = if let Some(repr) = &ty_def.repr {
        let kind = match repr {
            IdlRepr::Rust(_) => "Rust",
            IdlRepr::C(_) => "C",
            IdlRepr::Transparent => "transparent",
            _ => unimplemented!("{repr:?}"),
        };
        let kind = format_ident!("{kind}");

        let modifier = match repr {
            IdlRepr::Rust(modifier) | IdlRepr::C(modifier) => {
                let packed = modifier.packed.then(|| quote!(packed)).unwrap_or_default();
                let align = modifier
                    .align
                    .map(Literal::usize_unsuffixed)
                    .map(|align| quote!(align(#align)))
                    .unwrap_or_default();

                if packed.is_empty() {
                    align
                } else if align.is_empty() {
                    packed
                } else {
                    quote! { #packed, #align }
                }
            }
            _ => quote!(),
        };
        let modifier = if modifier.is_empty() {
            modifier
        } else {
            quote! { , #modifier }
        };

        quote! { #[repr(#kind #modifier)] }
    } else {
        quote!()
    };

    let ty = match &ty_def.ty {
        IdlTypeDefTy::Struct { fields } => {
            let declare_struct = quote! { pub struct #name #generics };
            handle_defined_fields(
                fields.as_ref(),
                || quote! { #declare_struct; },
                |fields| {
                    let fields = fields.iter().map(|field| {
                        let name = format_ident!("{}", field.name);
                        let ty = convert_idl_type_to_syn_type(&field.ty);
                        quote! { pub #name : #ty }
                    });
                    quote! {
                        #declare_struct {
                            #(#fields,)*
                        }
                    }
                },
                |tys| {
                    let tys = tys
                        .iter()
                        .map(convert_idl_type_to_syn_type)
                        .map(|ty| quote! { pub #ty });

                    quote! {
                        #declare_struct (#(#tys,)*);
                    }
                },
            )
        }
        IdlTypeDefTy::Enum { variants } => {
            let variants = variants.iter().map(|variant| {
                let variant_name = format_ident!("{}", variant.name);
                handle_defined_fields(
                    variant.fields.as_ref(),
                    || quote! { #variant_name },
                    |fields| {
                        let fields = fields.iter().map(|field| {
                            let name = format_ident!("{}", field.name);
                            let ty = convert_idl_type_to_syn_type(&field.ty);
                            quote! { #name : #ty }
                        });
                        quote! {
                            #variant_name {
                                #(#fields,)*
                            }
                        }
                    },
                    |tys| {
                        let tys = tys.iter().map(convert_idl_type_to_syn_type);
                        quote! {
                            #variant_name (#(#tys,)*)
                        }
                    },
                )
            });

            quote! {
                pub enum #name #generics {
                    #(#variants,)*
                }
            }
        }
        IdlTypeDefTy::Type { alias } => {
            let alias = convert_idl_type_to_syn_type(alias);
            quote! { pub type #name = #alias; }
        }
    };

    quote! {
        #docs
        #attrs
        #repr
        #ty
    }
}

fn can_derive_copy(ty_def: &IdlTypeDef, ty_defs: &[IdlTypeDef]) -> bool {
    match &ty_def.ty {
        IdlTypeDefTy::Struct { fields } => {
            can_derive_common(fields.as_ref(), ty_defs, can_derive_copy_ty)
        }
        IdlTypeDefTy::Enum { variants } => variants
            .iter()
            .all(|variant| can_derive_common(variant.fields.as_ref(), ty_defs, can_derive_copy_ty)),
        IdlTypeDefTy::Type { alias } => can_derive_copy_ty(alias, ty_defs),
    }
}

fn can_derive_default(ty_def: &IdlTypeDef, ty_defs: &[IdlTypeDef]) -> bool {
    match &ty_def.ty {
        IdlTypeDefTy::Struct { fields } => {
            can_derive_common(fields.as_ref(), ty_defs, can_derive_default_ty)
        }
        // TODO: Consider storing the default enum variant in IDL
        IdlTypeDefTy::Enum { .. } => false,
        IdlTypeDefTy::Type { alias } => can_derive_default_ty(alias, ty_defs),
    }
}

fn can_derive_copy_ty(ty: &IdlType, ty_defs: &[IdlTypeDef]) -> bool {
    match ty {
        IdlType::Option(inner) => can_derive_copy_ty(inner, ty_defs),
        IdlType::Array(inner, len) => {
            if !can_derive_copy_ty(inner, ty_defs) {
                return false;
            }

            match len {
                IdlArrayLen::Value(_) => true,
                IdlArrayLen::Generic(_) => false,
            }
        }
        IdlType::Defined { name, .. } => ty_defs
            .iter()
            .find(|ty_def| &ty_def.name == name)
            .map(|ty_def| can_derive_copy(ty_def, ty_defs))
            .expect("Type def must exist"),
        IdlType::Bytes | IdlType::String | IdlType::Vec(_) | IdlType::Generic(_) => false,
        _ => true,
    }
}

fn can_derive_default_ty(ty: &IdlType, ty_defs: &[IdlTypeDef]) -> bool {
    match ty {
        IdlType::Option(inner) => can_derive_default_ty(inner, ty_defs),
        IdlType::Vec(inner) => can_derive_default_ty(inner, ty_defs),
        IdlType::Array(inner, len) => {
            if !can_derive_default_ty(inner, ty_defs) {
                return false;
            }

            match len {
                IdlArrayLen::Value(len) => *len <= 32,
                IdlArrayLen::Generic(_) => false,
            }
        }
        IdlType::Defined { name, .. } => ty_defs
            .iter()
            .find(|ty_def| &ty_def.name == name)
            .map(|ty_def| can_derive_default(ty_def, ty_defs))
            .expect("Type def must exist"),
        IdlType::Generic(_) => false,
        _ => true,
    }
}

fn can_derive_common(
    fields: Option<&IdlDefinedFields>,
    ty_defs: &[IdlTypeDef],
    can_derive_ty: fn(&IdlType, &[IdlTypeDef]) -> bool,
) -> bool {
    handle_defined_fields(
        fields,
        || true,
        |fields| {
            fields
                .iter()
                .map(|field| &field.ty)
                .all(|ty| can_derive_ty(ty, ty_defs))
        },
        |tys| tys.iter().all(|ty| can_derive_ty(ty, ty_defs)),
    )
}

fn handle_defined_fields<R>(
    fields: Option<&IdlDefinedFields>,
    unit_cb: impl Fn() -> R,
    named_cb: impl Fn(&[IdlField]) -> R,
    tuple_cb: impl Fn(&[IdlType]) -> R,
) -> R {
    match fields {
        Some(fields) => match fields {
            IdlDefinedFields::Named(fields) => named_cb(fields),
            IdlDefinedFields::Tuple(tys) => tuple_cb(tys),
        },
        _ => unit_cb(),
    }
}
