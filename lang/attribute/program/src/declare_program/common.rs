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
        let debug_attr = can_derive_debug(ty_def, ty_defs)
            .then_some(quote!(#[derive(Debug)]))
            .unwrap_or_default();

        let default_attr =
            can_derive_default(ty_def, ty_defs).then_some(quote!(#[derive(Default)]));

        let ser_attr = match &ty_def.serialization {
            IdlSerialization::Borsh => quote!(#[derive(AnchorSerialize, AnchorDeserialize)]),
            IdlSerialization::Bytemuck => quote!(#[zero_copy]),
            IdlSerialization::BytemuckUnsafe => quote!(#[zero_copy(unsafe)]),
            _ => unimplemented!("{:?}", ty_def.serialization),
        };

        let clone_attr = can_derive_clone(ty_def, ty_defs)
            .then_some(quote!(#[derive(Clone)]))
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

    let repr = ty_def.repr.as_ref().map(|repr| {
        let kind = match repr {
            IdlRepr::Rust(_) => "Rust",
            IdlRepr::C(_) => "C",
            IdlRepr::Transparent => "transparent",
            _ => unimplemented!("{repr:?}"),
        };
        let kind = format_ident!("{kind}");

        let modifier = match repr {
            IdlRepr::Rust(modifier) | IdlRepr::C(modifier) => {
                let packed = modifier.packed.then_some(quote!(packed));
                let align = modifier
                    .align
                    .map(Literal::usize_unsuffixed)
                    .map(|align| quote!(align(#align)));

                match (packed, align) {
                    (None, None) => None,
                    (Some(p), None) => Some(quote!(#p)),
                    (None, Some(a)) => Some(quote!(#a)),
                    (Some(p), Some(a)) => Some(quote!(#p, #a)),
                }
            }
            _ => None,
        }
        .map(|m| quote!(, #m));
        quote! { #[repr(#kind #modifier)] }
    });

    match &ty_def.ty {
        IdlTypeDefTy::Struct { fields } => {
            let declare_struct = quote! { pub struct #name #generics };
            let ty = handle_defined_fields(
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
            );

            quote! {
                #docs
                #attrs
                #repr
                #ty
            }
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
                #docs
                #attrs
                #repr
                pub enum #name #generics {
                    #(#variants,)*
                }
            }
        }
        IdlTypeDefTy::Type { alias } => {
            let alias = convert_idl_type_to_syn_type(alias);
            quote! {
                #docs
                pub type #name = #alias;
            }
        }
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

fn can_derive_clone(ty_def: &IdlTypeDef, ty_defs: &[IdlTypeDef]) -> bool {
    match &ty_def.ty {
        IdlTypeDefTy::Struct { fields } => {
            can_derive_common(fields.as_ref(), ty_defs, can_derive_clone_ty)
        }
        IdlTypeDefTy::Enum { variants } => variants.iter().all(|variant| {
            can_derive_common(variant.fields.as_ref(), ty_defs, can_derive_clone_ty)
        }),
        IdlTypeDefTy::Type { alias } => can_derive_clone_ty(alias, ty_defs),
    }
}

fn can_derive_debug(ty_def: &IdlTypeDef, ty_defs: &[IdlTypeDef]) -> bool {
    match &ty_def.ty {
        IdlTypeDefTy::Struct { fields } => {
            can_derive_common(fields.as_ref(), ty_defs, can_derive_debug_ty)
        }
        IdlTypeDefTy::Enum { variants } => variants.iter().all(|variant| {
            can_derive_common(variant.fields.as_ref(), ty_defs, can_derive_debug_ty)
        }),
        IdlTypeDefTy::Type { alias } => can_derive_debug_ty(alias, ty_defs),
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

pub fn can_derive_copy_ty(ty: &IdlType, ty_defs: &[IdlTypeDef]) -> bool {
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

pub fn can_derive_clone_ty(ty: &IdlType, ty_defs: &[IdlTypeDef]) -> bool {
    match ty {
        IdlType::Option(inner) => can_derive_clone_ty(inner, ty_defs),
        IdlType::Vec(inner) => can_derive_clone_ty(inner, ty_defs),
        IdlType::Array(inner, _) => can_derive_clone_ty(inner, ty_defs),
        IdlType::Defined { name, .. } => ty_defs
            .iter()
            .find(|ty_def| &ty_def.name == name)
            .map(|ty_def| can_derive_clone(ty_def, ty_defs))
            .expect("Type def must exist"),
        IdlType::Generic(_) => false,
        _ => true,
    }
}

pub fn can_derive_debug_ty(ty: &IdlType, ty_defs: &[IdlTypeDef]) -> bool {
    match ty {
        IdlType::Option(inner) => can_derive_debug_ty(inner, ty_defs),
        IdlType::Vec(inner) => can_derive_debug_ty(inner, ty_defs),
        IdlType::Array(inner, _) => can_derive_debug_ty(inner, ty_defs),
        IdlType::Defined { name, .. } => ty_defs
            .iter()
            .find(|ty_def| &ty_def.name == name)
            .map(|ty_def| can_derive_debug(ty_def, ty_defs))
            .expect("Type def must exist"),
        IdlType::Generic(_) => false,
        _ => true,
    }
}

pub fn can_derive_default_ty(ty: &IdlType, ty_defs: &[IdlTypeDef]) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang_idl::types::{
        IdlArrayLen, IdlDefinedFields, IdlField, IdlGenericArg, IdlSerialization, IdlType,
        IdlTypeDef, IdlTypeDefTy,
    };

    fn create_test_idl_types() -> Vec<IdlTypeDef> {
        vec![
            // Simple struct with copyable types
            IdlTypeDef {
                name: "SimpleStruct".to_string(),
                ty: IdlTypeDefTy::Struct {
                    fields: Some(IdlDefinedFields::Named(vec![IdlField {
                        name: "value".to_string(),
                        ty: IdlType::U64,
                        docs: vec![],
                    }])),
                },
                generics: vec![],
                docs: vec![],
                serialization: IdlSerialization::Borsh,
                repr: None,
            },
            // Struct with non-copyable types
            IdlTypeDef {
                name: "NonCopyStruct".to_string(),
                ty: IdlTypeDefTy::Struct {
                    fields: Some(IdlDefinedFields::Named(vec![IdlField {
                        name: "data".to_string(),
                        ty: IdlType::String,
                        docs: vec![],
                    }])),
                },
                generics: vec![],
                docs: vec![],
                serialization: IdlSerialization::Borsh,
                repr: None,
            },
            // Enum with copyable variants
            IdlTypeDef {
                name: "SimpleEnum".to_string(),
                ty: IdlTypeDefTy::Enum {
                    variants: vec![
                        anchor_lang_idl::types::IdlEnumVariant {
                            name: "Variant1".to_string(),
                            fields: None,
                        },
                        anchor_lang_idl::types::IdlEnumVariant {
                            name: "Variant2".to_string(),
                            fields: Some(IdlDefinedFields::Named(vec![IdlField {
                                name: "value".to_string(),
                                ty: IdlType::U32,
                                docs: vec![],
                            }])),
                        },
                    ],
                },
                generics: vec![],
                docs: vec![],
                serialization: IdlSerialization::Borsh,
                repr: None,
            },
            // Type alias
            IdlTypeDef {
                name: "TypeAlias".to_string(),
                ty: IdlTypeDefTy::Type {
                    alias: IdlType::U128,
                },
                generics: vec![],
                docs: vec![],
                serialization: IdlSerialization::Borsh,
                repr: None,
            },
        ]
    }

    #[test]
    fn test_can_derive_copy_ty() {
        let ty_defs = create_test_idl_types();

        // Test basic copyable types
        assert!(can_derive_copy_ty(&IdlType::U8, &ty_defs));
        assert!(can_derive_copy_ty(&IdlType::U64, &ty_defs));
        assert!(can_derive_copy_ty(&IdlType::Bool, &ty_defs));
        assert!(can_derive_copy_ty(&IdlType::Pubkey, &ty_defs));

        // Test non-copyable types
        assert!(!can_derive_copy_ty(&IdlType::String, &ty_defs));
        assert!(!can_derive_copy_ty(&IdlType::Bytes, &ty_defs));
        assert!(!can_derive_copy_ty(
            &IdlType::Vec(Box::new(IdlType::U8)),
            &ty_defs
        ));

        // Test Option with copyable inner type
        assert!(can_derive_copy_ty(
            &IdlType::Option(Box::new(IdlType::U64)),
            &ty_defs
        ));
        assert!(!can_derive_copy_ty(
            &IdlType::Option(Box::new(IdlType::String)),
            &ty_defs
        ));

        // Test Array with copyable inner type
        assert!(can_derive_copy_ty(
            &IdlType::Array(Box::new(IdlType::U8), IdlArrayLen::Value(10)),
            &ty_defs
        ));
        assert!(!can_derive_copy_ty(
            &IdlType::Array(Box::new(IdlType::String), IdlArrayLen::Value(5)),
            &ty_defs
        ));

        // Test Array with generic length (should not be copyable)
        assert!(!can_derive_copy_ty(
            &IdlType::Array(Box::new(IdlType::U8), IdlArrayLen::Generic("N".to_string())),
            &ty_defs
        ));

        // Test defined types
        assert!(can_derive_copy_ty(
            &IdlType::Defined {
                name: "SimpleStruct".to_string(),
                generics: vec![],
            },
            &ty_defs
        ));
        assert!(!can_derive_copy_ty(
            &IdlType::Defined {
                name: "NonCopyStruct".to_string(),
                generics: vec![],
            },
            &ty_defs
        ));

        // Test generic types (should not be copyable)
        assert!(!can_derive_copy_ty(
            &IdlType::Generic("T".to_string()),
            &ty_defs
        ));
    }

    #[test]
    fn test_can_derive_clone_ty() {
        let ty_defs = create_test_idl_types();

        // Test basic cloneable types
        assert!(can_derive_clone_ty(&IdlType::U8, &ty_defs));
        assert!(can_derive_clone_ty(&IdlType::String, &ty_defs));
        assert!(can_derive_clone_ty(&IdlType::Bytes, &ty_defs));

        // Test Vec with cloneable inner type
        assert!(can_derive_clone_ty(
            &IdlType::Vec(Box::new(IdlType::U8)),
            &ty_defs
        ));
        assert!(can_derive_clone_ty(
            &IdlType::Vec(Box::new(IdlType::String)),
            &ty_defs
        ));

        // Test Array with cloneable inner type
        assert!(can_derive_clone_ty(
            &IdlType::Array(Box::new(IdlType::U8), IdlArrayLen::Value(10)),
            &ty_defs
        ));

        // Test Option with cloneable inner type
        assert!(can_derive_clone_ty(
            &IdlType::Option(Box::new(IdlType::String)),
            &ty_defs
        ));

        // Test defined types
        assert!(can_derive_clone_ty(
            &IdlType::Defined {
                name: "SimpleStruct".to_string(),
                generics: vec![],
            },
            &ty_defs
        ));
        assert!(can_derive_clone_ty(
            &IdlType::Defined {
                name: "NonCopyStruct".to_string(),
                generics: vec![],
            },
            &ty_defs
        ));

        // Test generic types (should not be cloneable)
        assert!(!can_derive_clone_ty(
            &IdlType::Generic("T".to_string()),
            &ty_defs
        ));
    }

    #[test]
    fn test_can_derive_debug_ty() {
        let ty_defs = create_test_idl_types();

        // Test basic debuggable types
        assert!(can_derive_debug_ty(&IdlType::U8, &ty_defs));
        assert!(can_derive_debug_ty(&IdlType::String, &ty_defs));
        assert!(can_derive_debug_ty(&IdlType::Bytes, &ty_defs));

        // Test Vec with debuggable inner type
        assert!(can_derive_debug_ty(
            &IdlType::Vec(Box::new(IdlType::U8)),
            &ty_defs
        ));

        // Test Array with debuggable inner type
        assert!(can_derive_debug_ty(
            &IdlType::Array(Box::new(IdlType::U8), IdlArrayLen::Value(10)),
            &ty_defs
        ));

        // Test Option with debuggable inner type
        assert!(can_derive_debug_ty(
            &IdlType::Option(Box::new(IdlType::String)),
            &ty_defs
        ));

        // Test defined types
        assert!(can_derive_debug_ty(
            &IdlType::Defined {
                name: "SimpleStruct".to_string(),
                generics: vec![],
            },
            &ty_defs
        ));
        assert!(can_derive_debug_ty(
            &IdlType::Defined {
                name: "NonCopyStruct".to_string(),
                generics: vec![],
            },
            &ty_defs
        ));

        // Test generic types (should not be debuggable)
        assert!(!can_derive_debug_ty(
            &IdlType::Generic("T".to_string()),
            &ty_defs
        ));
    }

    #[test]
    fn test_can_derive_default_ty() {
        let ty_defs = create_test_idl_types();

        // Test basic defaultable types
        assert!(can_derive_default_ty(&IdlType::U8, &ty_defs));
        assert!(can_derive_default_ty(&IdlType::String, &ty_defs));
        assert!(can_derive_default_ty(&IdlType::Bool, &ty_defs));

        // Test Vec (should be defaultable)
        assert!(can_derive_default_ty(
            &IdlType::Vec(Box::new(IdlType::U8)),
            &ty_defs
        ));

        // Test Array with small fixed size (should be defaultable)
        assert!(can_derive_default_ty(
            &IdlType::Array(Box::new(IdlType::U8), IdlArrayLen::Value(10)),
            &ty_defs
        ));

        // Test Array with large fixed size (should not be defaultable)
        assert!(!can_derive_default_ty(
            &IdlType::Array(Box::new(IdlType::U8), IdlArrayLen::Value(100)),
            &ty_defs
        ));

        // Test Array with generic length (should not be defaultable)
        assert!(!can_derive_default_ty(
            &IdlType::Array(Box::new(IdlType::U8), IdlArrayLen::Generic("N".to_string())),
            &ty_defs
        ));

        // Test Option with defaultable inner type
        assert!(can_derive_default_ty(
            &IdlType::Option(Box::new(IdlType::String)),
            &ty_defs
        ));

        // Test defined types
        assert!(can_derive_default_ty(
            &IdlType::Defined {
                name: "SimpleStruct".to_string(),
                generics: vec![],
            },
            &ty_defs
        ));
        assert!(can_derive_default_ty(
            &IdlType::Defined {
                name: "NonCopyStruct".to_string(),
                generics: vec![],
            },
            &ty_defs
        ));

        // Test generic types (should not be defaultable)
        assert!(!can_derive_default_ty(
            &IdlType::Generic("T".to_string()),
            &ty_defs
        ));
    }

    #[test]
    fn test_can_derive_copy() {
        let ty_defs = create_test_idl_types();

        // Test struct with copyable fields
        let simple_struct = &ty_defs[0];
        assert!(can_derive_copy(simple_struct, &ty_defs));

        // Test struct with non-copyable fields
        let non_copy_struct = &ty_defs[1];
        assert!(!can_derive_copy(non_copy_struct, &ty_defs));

        // Test enum with copyable variants
        let simple_enum = &ty_defs[2];
        assert!(can_derive_copy(simple_enum, &ty_defs));

        // Test type alias
        let type_alias = &ty_defs[3];
        assert!(can_derive_copy(type_alias, &ty_defs));
    }

    #[test]
    fn test_can_derive_clone() {
        let ty_defs = create_test_idl_types();

        // Test struct with cloneable fields
        let simple_struct = &ty_defs[0];
        assert!(can_derive_clone(simple_struct, &ty_defs));

        // Test struct with cloneable fields (String is cloneable)
        let non_copy_struct = &ty_defs[1];
        assert!(can_derive_clone(non_copy_struct, &ty_defs));

        // Test enum with cloneable variants
        let simple_enum = &ty_defs[2];
        assert!(can_derive_clone(simple_enum, &ty_defs));

        // Test type alias
        let type_alias = &ty_defs[3];
        assert!(can_derive_clone(type_alias, &ty_defs));
    }

    #[test]
    fn test_can_derive_debug() {
        let ty_defs = create_test_idl_types();

        // Test struct with debuggable fields
        let simple_struct = &ty_defs[0];
        assert!(can_derive_debug(simple_struct, &ty_defs));

        // Test struct with debuggable fields
        let non_copy_struct = &ty_defs[1];
        assert!(can_derive_debug(non_copy_struct, &ty_defs));

        // Test enum with debuggable variants
        let simple_enum = &ty_defs[2];
        assert!(can_derive_debug(simple_enum, &ty_defs));

        // Test type alias
        let type_alias = &ty_defs[3];
        assert!(can_derive_debug(type_alias, &ty_defs));
    }

    #[test]
    fn test_can_derive_default() {
        let ty_defs = create_test_idl_types();

        // Test struct with defaultable fields
        let simple_struct = &ty_defs[0];
        assert!(can_derive_default(simple_struct, &ty_defs));

        // Test struct with defaultable fields
        let non_copy_struct = &ty_defs[1];
        assert!(can_derive_default(non_copy_struct, &ty_defs));

        // Test enum (should not be defaultable)
        let simple_enum = &ty_defs[2];
        assert!(!can_derive_default(simple_enum, &ty_defs));

        // Test type alias
        let type_alias = &ty_defs[3];
        assert!(can_derive_default(type_alias, &ty_defs));
    }

    #[test]
    fn test_convert_idl_type_to_str() {
        // Test basic types
        assert_eq!(convert_idl_type_to_str(&IdlType::Bool), "bool");
        assert_eq!(convert_idl_type_to_str(&IdlType::U8), "u8");
        assert_eq!(convert_idl_type_to_str(&IdlType::U64), "u64");
        assert_eq!(convert_idl_type_to_str(&IdlType::String), "String");
        assert_eq!(convert_idl_type_to_str(&IdlType::Pubkey), "Pubkey");

        // Test Option
        assert_eq!(
            convert_idl_type_to_str(&IdlType::Option(Box::new(IdlType::U64))),
            "Option<u64>"
        );

        // Test Vec
        assert_eq!(
            convert_idl_type_to_str(&IdlType::Vec(Box::new(IdlType::String))),
            "Vec<String>"
        );

        // Test Array with value length
        assert_eq!(
            convert_idl_type_to_str(&IdlType::Array(
                Box::new(IdlType::U8),
                IdlArrayLen::Value(10)
            )),
            "[u8; 10]"
        );

        // Test Array with generic length
        assert_eq!(
            convert_idl_type_to_str(&IdlType::Array(
                Box::new(IdlType::U8),
                IdlArrayLen::Generic("N".to_string())
            )),
            "[u8; N]"
        );

        // Test defined type without generics
        assert_eq!(
            convert_idl_type_to_str(&IdlType::Defined {
                name: "MyStruct".to_string(),
                generics: vec![],
            }),
            "MyStruct"
        );

        // Test defined type with generics
        assert_eq!(
            convert_idl_type_to_str(&IdlType::Defined {
                name: "MyStruct".to_string(),
                generics: vec![
                    IdlGenericArg::Type { ty: IdlType::U64 },
                    IdlGenericArg::Const {
                        value: "10".to_string()
                    },
                ],
            }),
            "MyStruct<u64,10>"
        );

        // Test generic type
        assert_eq!(
            convert_idl_type_to_str(&IdlType::Generic("T".to_string())),
            "T"
        );
    }

    #[test]
    fn test_gen_discriminator() {
        let disc = [1, 2, 3, 4, 5, 6, 7, 8];
        let result = gen_discriminator(&disc);
        let expected = quote! { [1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8] };
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn test_gen_docs() {
        let docs = vec!["First line".to_string(), "Second line".to_string()];
        let result = gen_docs(&docs);
        let expected = quote! {
            #[doc = " First line"]
            #[doc = " Second line"]
        };
        assert_eq!(result.to_string(), expected.to_string());

        // Test empty docs
        let empty_docs = vec![];
        let result = gen_docs(&empty_docs);
        let expected = quote! {};
        assert_eq!(result.to_string(), expected.to_string());
    }
}
