pub use serde_json;

use crate::{parser::docs, AccountField, AccountsStruct, Error, Program};
use heck::MixedCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, ItemEnum, ItemStruct};

/// A trait that types must implement in order to generate the IDL via compilation.
///
/// This trait is automatically implemented for Anchor all types that use the `AnchorSerialize`
/// proc macro. Note that manually implementing the `AnchorSerialize` trait will **NOT** have the
/// same effect.
///
/// Types that don't implement this trait will cause a compile error during the IDL generation.
///
/// The methods have default implementation that allows the program to compile but the type will
/// **NOT** be included in the IDL.
pub trait IdlBuild {
    /// Returns the full module path of the type.
    fn __anchor_private_full_path() -> String {
        String::default()
    }

    /// Returns the IDL type definition of the type or `None` if it doesn't exist.
    fn __anchor_private_gen_idl_type() -> Option<super::types::IdlTypeDefinition> {
        None
    }

    /// Insert the type definition to the defined types hashmap.
    fn __anchor_private_insert_idl_defined(
        _defined_types: &mut std::collections::HashMap<String, super::types::IdlTypeDefinition>,
    ) {
    }
}

#[inline(always)]
fn get_module_paths() -> (TokenStream, TokenStream) {
    (
        quote!(anchor_lang::anchor_syn::idl::types),
        quote!(anchor_lang::anchor_syn::idl::build::serde_json),
    )
}

#[inline(always)]
pub fn get_no_docs() -> bool {
    std::option_env!("ANCHOR_IDL_BUILD_NO_DOCS")
        .map(|val| val == "TRUE")
        .unwrap_or(false)
}

#[inline(always)]
pub fn get_seeds_feature() -> bool {
    std::option_env!("ANCHOR_IDL_BUILD_SEEDS_FEATURE")
        .map(|val| val == "TRUE")
        .unwrap_or(false)
}

// Returns TokenStream for IdlType enum and the syn::TypePath for the defined
// type if any.
// Returns Err when the type wasn't parsed successfully.
#[allow(clippy::result_unit_err)]
pub fn idl_type_ts_from_syn_type(
    ty: &syn::Type,
    type_params: &Vec<Ident>,
) -> Result<(TokenStream, Vec<syn::TypePath>), ()> {
    let (idl, _) = get_module_paths();

    fn the_only_segment_is(path: &syn::TypePath, cmp: &str) -> bool {
        if path.path.segments.len() != 1 {
            return false;
        };
        return path.path.segments.first().unwrap().ident == cmp;
    }

    // Foo<first::path, second::path> -> first::path
    fn get_first_angle_bracketed_path_arg(segment: &syn::PathSegment) -> Option<&syn::Type> {
        match &segment.arguments {
            syn::PathArguments::AngleBracketed(arguments) => match arguments.args.first() {
                Some(syn::GenericArgument::Type(ty)) => Some(ty),
                _ => None,
            },
            _ => None,
        }
    }

    match ty {
        syn::Type::Path(path) if the_only_segment_is(path, "bool") => {
            Ok((quote! { #idl::IdlType::Bool }, vec![]))
        }
        syn::Type::Path(path) if the_only_segment_is(path, "u8") => {
            Ok((quote! { #idl::IdlType::U8 }, vec![]))
        }
        syn::Type::Path(path) if the_only_segment_is(path, "i8") => {
            Ok((quote! { #idl::IdlType::I8 }, vec![]))
        }
        syn::Type::Path(path) if the_only_segment_is(path, "u16") => {
            Ok((quote! { #idl::IdlType::U16 }, vec![]))
        }
        syn::Type::Path(path) if the_only_segment_is(path, "i16") => {
            Ok((quote! { #idl::IdlType::I16 }, vec![]))
        }
        syn::Type::Path(path) if the_only_segment_is(path, "u32") => {
            Ok((quote! { #idl::IdlType::U32 }, vec![]))
        }
        syn::Type::Path(path) if the_only_segment_is(path, "i32") => {
            Ok((quote! { #idl::IdlType::I32 }, vec![]))
        }
        syn::Type::Path(path) if the_only_segment_is(path, "f32") => {
            Ok((quote! { #idl::IdlType::F32 }, vec![]))
        }
        syn::Type::Path(path) if the_only_segment_is(path, "u64") => {
            Ok((quote! { #idl::IdlType::U64 }, vec![]))
        }
        syn::Type::Path(path) if the_only_segment_is(path, "i64") => {
            Ok((quote! { #idl::IdlType::I64 }, vec![]))
        }
        syn::Type::Path(path) if the_only_segment_is(path, "f64") => {
            Ok((quote! { #idl::IdlType::F64 }, vec![]))
        }
        syn::Type::Path(path) if the_only_segment_is(path, "u128") => {
            Ok((quote! { #idl::IdlType::U128 }, vec![]))
        }
        syn::Type::Path(path) if the_only_segment_is(path, "i128") => {
            Ok((quote! { #idl::IdlType::I128 }, vec![]))
        }
        syn::Type::Path(path)
            if the_only_segment_is(path, "String") || the_only_segment_is(path, "&str") =>
        {
            Ok((quote! { #idl::IdlType::String }, vec![]))
        }
        syn::Type::Path(path) if the_only_segment_is(path, "Pubkey") => {
            Ok((quote! { #idl::IdlType::PublicKey }, vec![]))
        }
        syn::Type::Path(path) if the_only_segment_is(path, "Vec") => {
            let segment = path.path.segments.first().unwrap();
            let arg = match get_first_angle_bracketed_path_arg(segment) {
                Some(arg) => arg,
                None => unreachable!("Vec arguments can only be of AngleBracketed variant"),
            };
            match arg {
                syn::Type::Path(path) if the_only_segment_is(path, "u8") => {
                    return Ok((quote! {#idl::IdlType::Bytes}, vec![]));
                }
                _ => (),
            };
            let (inner, defined) = idl_type_ts_from_syn_type(arg, type_params)?;
            Ok((quote! { #idl::IdlType::Vec(Box::new(#inner)) }, defined))
        }
        syn::Type::Path(path) if the_only_segment_is(path, "Option") => {
            let segment = path.path.segments.first().unwrap();
            let arg = match get_first_angle_bracketed_path_arg(segment) {
                Some(arg) => arg,
                None => unreachable!("Option arguments can only be of AngleBracketed variant"),
            };
            let (inner, defined) = idl_type_ts_from_syn_type(arg, type_params)?;
            Ok((quote! { #idl::IdlType::Option(Box::new(#inner)) }, defined))
        }
        syn::Type::Path(path) if the_only_segment_is(path, "Box") => {
            let segment = path.path.segments.first().unwrap();
            let arg = match get_first_angle_bracketed_path_arg(segment) {
                Some(arg) => arg,
                None => unreachable!("Box arguments can only be of AngleBracketed variant"),
            };
            let (ts, defined) = idl_type_ts_from_syn_type(arg, type_params)?;
            Ok((quote! { #ts }, defined))
        }
        syn::Type::Array(arr) => {
            let len = arr.len.clone();
            let len_is_generic = type_params.iter().any(|param| match len {
                syn::Expr::Path(ref path) => path.path.is_ident(param),
                _ => false,
            });

            let (inner, defined) = idl_type_ts_from_syn_type(&arr.elem, type_params)?;

            if len_is_generic {
                match len {
                    syn::Expr::Path(ref len) => {
                        let len = len.path.get_ident().unwrap().to_string();
                        Ok((
                            quote! { #idl::IdlType::GenericLenArray(Box::new(#inner), #len.into()) },
                            defined,
                        ))
                    }
                    _ => unreachable!("Array length can only be a generic parameter"),
                }
            } else {
                Ok((
                    quote! { #idl::IdlType::Array(Box::new(#inner), #len) },
                    defined,
                ))
            }
        }
        syn::Type::Path(path) => {
            let is_generic_param = type_params.iter().any(|param| path.path.is_ident(param));

            if is_generic_param {
                let generic = format!("{}", path.path.get_ident().unwrap());
                Ok((quote! { #idl::IdlType::Generic(#generic.into()) }, vec![]))
            } else {
                let mut params = vec![];
                let mut defined = vec![path.clone()];

                if let Some(segment) = &path.path.segments.last() {
                    if let syn::PathArguments::AngleBracketed(ref args) = segment.arguments {
                        for arg in &args.args {
                            match arg {
                                syn::GenericArgument::Type(ty) => {
                                    let (ts, def) = idl_type_ts_from_syn_type(ty, type_params)?;
                                    params.push(quote! { #idl::IdlDefinedTypeArg::Type(#ts) });
                                    defined.extend(def);
                                }
                                syn::GenericArgument::Const(c) => params.push(
                                    quote! { #idl::IdlDefinedTypeArg::Value(format!("{}", #c))},
                                ),
                                _ => (),
                            }
                        }
                    }
                }

                if !params.is_empty() {
                    let params = quote! { vec![#(#params),*] };
                    Ok((
                        quote! { #idl::IdlType::DefinedWithTypeArgs {
                            name: <#path>::__anchor_private_full_path(),
                            args: #params
                        } },
                        defined,
                    ))
                } else {
                    Ok((
                        quote! { #idl::IdlType::Defined(<#path>::__anchor_private_full_path()) },
                        vec![path.clone()],
                    ))
                }
            }
        }
        syn::Type::Reference(reference) => match reference.elem.as_ref() {
            syn::Type::Slice(slice) if matches!(&*slice.elem, syn::Type::Path(path) if the_only_segment_is(path, "u8")) =>
            {
                return Ok((quote! {#idl::IdlType::Bytes}, vec![]));
            }
            _ => panic!("Reference types other than byte slice(`&[u8]`) are not allowed"),
        },
        _ => Err(()),
    }
}

// Returns TokenStream for IdlField struct and the syn::TypePath for the defined
// type if any.
// Returns Err when the type wasn't parsed successfully
#[allow(clippy::result_unit_err)]
pub fn idl_field_ts_from_syn_field(
    field: &syn::Field,
    no_docs: bool,
    type_params: &Vec<syn::Ident>,
) -> Result<(TokenStream, Vec<syn::TypePath>), ()> {
    let (idl, _) = get_module_paths();

    let name = field.ident.as_ref().unwrap().to_string().to_mixed_case();
    let docs = match docs::parse(&field.attrs) {
        Some(docs) if !no_docs => quote! {Some(vec![#(#docs.into()),*])},
        _ => quote! {None},
    };
    let (ty, defined) = idl_type_ts_from_syn_type(&field.ty, type_params)?;

    Ok((
        quote! {
            #idl::IdlField {
                name: #name.into(),
                docs: #docs,
                ty: #ty,
            }
        },
        defined,
    ))
}

// Returns TokenStream for IdlEventField struct and the syn::TypePath for the defined
// type if any.
// Returns Err when the type wasn't parsed successfully
#[allow(clippy::result_unit_err)]
pub fn idl_event_field_ts_from_syn_field(
    field: &syn::Field,
) -> Result<(TokenStream, Vec<syn::TypePath>), ()> {
    let (idl, _) = get_module_paths();

    let name = field.ident.as_ref().unwrap().to_string().to_mixed_case();
    let (ty, defined) = idl_type_ts_from_syn_type(&field.ty, &vec![])?;

    let index: bool = field
        .attrs
        .get(0)
        .and_then(|attr| attr.path.segments.first())
        .map(|segment| segment.ident == "index")
        .unwrap_or(false);

    Ok((
        quote! {
            #idl::IdlEventField {
                name: #name.into(),
                ty: #ty,
                index: #index,
            }
        },
        defined,
    ))
}

// Returns TokenStream for IdlTypeDefinitionTy::Struct and Vec<&syn::TypePath>
// for the defined types if any.
// Returns Err if any of the fields weren't parsed successfully.
#[allow(clippy::result_unit_err)]
pub fn idl_type_definition_ts_from_syn_struct(
    item_strct: &syn::ItemStruct,
    no_docs: bool,
) -> Result<(TokenStream, Vec<syn::TypePath>), ()> {
    let (idl, _) = get_module_paths();

    let docs = match docs::parse(&item_strct.attrs) {
        Some(docs) if !no_docs => quote! {Some(vec![#(#docs.into()),*])},
        _ => quote! {None},
    };

    let type_params = item_strct
        .generics
        .params
        .iter()
        .filter_map(|p| match p {
            syn::GenericParam::Type(ty) => Some(ty.ident.clone()),
            syn::GenericParam::Const(c) => Some(c.ident.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let (fields, defined): (Vec<TokenStream>, Vec<Vec<syn::TypePath>>) = match &item_strct.fields {
        syn::Fields::Named(fields) => fields
            .named
            .iter()
            .map(|f: &syn::Field| idl_field_ts_from_syn_field(f, no_docs, &type_params))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .unzip::<_, _, Vec<_>, Vec<_>>(),
        _ => return Err(()),
    };
    let defined = defined
        .into_iter()
        .flatten()
        .collect::<Vec<syn::TypePath>>();

    let generics = if !type_params.is_empty() {
        let g: Vec<String> = type_params.iter().map(|id| id.to_string()).collect();
        quote! { Some(vec![#(#g.into()),*]) }
    } else {
        quote! { None }
    };

    Ok((
        quote! {
            #idl::IdlTypeDefinition {
                name: Self::__anchor_private_full_path(),
                generics: #generics,
                docs: #docs,
                ty: #idl::IdlTypeDefinitionTy::Struct{
                    fields: vec![
                        #(#fields),*
                    ]
                }
            },
        },
        defined,
    ))
}

// Returns TokenStream for IdlTypeDefinitionTy::Enum and the Vec<&syn::TypePath>
// for the defined types if any.
// Returns Err if any of the fields didn't parse successfully.
#[allow(clippy::result_unit_err)]
pub fn idl_type_definition_ts_from_syn_enum(
    enum_item: &syn::ItemEnum,
    no_docs: bool,
) -> Result<(TokenStream, Vec<syn::TypePath>), ()> {
    let (idl, _) = get_module_paths();

    let docs = match docs::parse(&enum_item.attrs) {
        Some(docs) if !no_docs => quote! {Some(vec![#(#docs.into()),*])},
        _ => quote! {None},
    };

    let type_params = enum_item
        .generics
        .params
        .iter()
        .filter_map(|p| match p {
            syn::GenericParam::Type(ty) => Some(ty.ident.clone()),
            syn::GenericParam::Const(c) => Some(c.ident.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();

    let (variants, defined): (Vec<TokenStream>, Vec<Vec<syn::TypePath>>) = enum_item.variants.iter().map(|variant: &syn::Variant| {
        let name = variant.ident.to_string();
        let (fields, defined): (TokenStream, Vec<syn::TypePath>) = match &variant.fields {
            syn::Fields::Unit => (quote!{None}, vec![]),
            syn::Fields::Unnamed(fields) => {
                let (types, defined) = fields.unnamed
                    .iter()
                    .map(|f| idl_type_ts_from_syn_type(&f.ty, &type_params))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .unzip::<TokenStream, Vec<syn::TypePath>, Vec<TokenStream>, Vec<Vec<syn::TypePath>>>();
                let defined = defined
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                (quote!{ Some(#idl::EnumFields::Tuple(vec![#(#types),*]))}, defined)
            }
            syn::Fields::Named(fields) => {
                let (fields, defined) = fields.named
                    .iter()
                    .map(|f| idl_field_ts_from_syn_field(f, no_docs, &type_params))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .unzip::<TokenStream, Vec<syn::TypePath>, Vec<TokenStream>, Vec<Vec<syn::TypePath>>>();
                let defined = defined
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                (quote!{ Some(#idl::EnumFields::Named(vec![#(#fields),*]))}, defined)
            }
        };

        Ok((quote!{ #idl::IdlEnumVariant{ name: #name.into(), fields: #fields }}, defined))
    })
    .collect::<Result<Vec<_>, _>>()?
    .into_iter()
    .unzip::<TokenStream, Vec<syn::TypePath>, Vec<TokenStream>, Vec<Vec<syn::TypePath>>>();

    let defined = defined
        .into_iter()
        .flatten()
        .collect::<Vec<syn::TypePath>>();

    let generics = if !type_params.is_empty() {
        let g: Vec<String> = type_params.iter().map(|id| id.to_string()).collect();
        quote! { Some(vec![#(#g.into()),*]) }
    } else {
        quote! { None }
    };

    Ok((
        quote! {
            #idl::IdlTypeDefinition {
                name: Self::__anchor_private_full_path(),
                generics: #generics,
                docs: #docs,
                ty: #idl::IdlTypeDefinitionTy::Enum{
                    variants: vec![
                        #(#variants),*
                    ]
                }
            }
        },
        defined,
    ))
}

pub fn idl_build_impl_skeleton(
    idl_type_definition_ts: TokenStream,
    insert_defined_ts: TokenStream,
    ident: &Ident,
    input_generics: &syn::Generics,
) -> TokenStream {
    let (idl, _) = get_module_paths();
    let name = ident.to_string();
    let (impl_generics, ty_generics, where_clause) = input_generics.split_for_impl();
    let idl_build_trait = quote! {anchor_lang::anchor_syn::idl::build::IdlBuild};

    quote! {
        impl #impl_generics #idl_build_trait for #ident #ty_generics #where_clause {
            fn __anchor_private_full_path() -> String {
                format!("{}::{}", std::module_path!(), #name)
            }

            fn __anchor_private_gen_idl_type() -> Option<#idl::IdlTypeDefinition> {
                #idl_type_definition_ts
            }

            fn __anchor_private_insert_idl_defined(
                defined_types: &mut std::collections::HashMap<String, #idl::IdlTypeDefinition>
            ) {
                #insert_defined_ts
            }
        }
    }
}

// generates the IDL generation impl for for a struct
pub fn gen_idl_build_impl_for_struct(strct: &ItemStruct, no_docs: bool) -> TokenStream {
    let idl_type_definition_ts: TokenStream;
    let insert_defined_ts: TokenStream;

    if let Ok((ts, defined)) = idl_type_definition_ts_from_syn_struct(strct, no_docs) {
        idl_type_definition_ts = quote! {Some(#ts)};
        insert_defined_ts = quote! {
            #({
                <#defined>::__anchor_private_insert_idl_defined(defined_types);

                let path = <#defined>::__anchor_private_full_path();
                <#defined>::__anchor_private_gen_idl_type()
                    .and_then(|ty| defined_types.insert(path, ty));
            });*
        };
    } else {
        idl_type_definition_ts = quote! {None};
        insert_defined_ts = quote! {};
    }

    let ident = &strct.ident;
    let input_generics = &strct.generics;

    idl_build_impl_skeleton(
        idl_type_definition_ts,
        insert_defined_ts,
        ident,
        input_generics,
    )
}

// generates the IDL generation impl for for an enum
pub fn gen_idl_build_impl_for_enum(enm: ItemEnum, no_docs: bool) -> TokenStream {
    let idl_type_definition_ts: TokenStream;
    let insert_defined_ts: TokenStream;

    if let Ok((ts, defined)) = idl_type_definition_ts_from_syn_enum(&enm, no_docs) {
        idl_type_definition_ts = quote! {Some(#ts)};
        insert_defined_ts = quote! {
            #({
                <#defined>::__anchor_private_insert_idl_defined(defined_types);

                let path = <#defined>::__anchor_private_full_path();
                <#defined>::__anchor_private_gen_idl_type()
                    .and_then(|ty| defined_types.insert(path, ty));
            });*
        };
    } else {
        idl_type_definition_ts = quote! {None};
        insert_defined_ts = quote! {};
    }

    let ident = &enm.ident;
    let input_generics = &enm.generics;

    idl_build_impl_skeleton(
        idl_type_definition_ts,
        insert_defined_ts,
        ident,
        input_generics,
    )
}

// generates the IDL generation impl for for an event
pub fn gen_idl_build_impl_for_event(event_strct: &ItemStruct) -> TokenStream {
    fn parse_fields(
        fields: &syn::FieldsNamed,
    ) -> Result<(Vec<TokenStream>, Vec<syn::TypePath>), ()> {
        let (fields, defined) = fields
            .named
            .iter()
            .map(idl_event_field_ts_from_syn_field)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .unzip::<_, _, Vec<_>, Vec<_>>();
        let defined = defined
            .into_iter()
            .flatten()
            .collect::<Vec<syn::TypePath>>();

        Ok((fields, defined))
    }

    let res = match &event_strct.fields {
        syn::Fields::Named(fields) => parse_fields(fields),
        _ => Err(()),
    };

    let (idl, _) = get_module_paths();
    let name = event_strct.ident.to_string();

    let (ret_ts, types_ts) = match res {
        Ok((fields, defined)) => {
            let ret_ts = quote! {
                Some(
                    #idl::IdlEvent {
                        name: #name.into(),
                        fields: vec![#(#fields),*],
                    }
                )
            };
            let types_ts = quote! {
                #({
                    <#defined>::__anchor_private_insert_idl_defined(defined_types);

                    let path = <#defined>::__anchor_private_full_path();
                    <#defined>::__anchor_private_gen_idl_type()
                        .and_then(|ty| defined_types.insert(path, ty));
                });*
            };
            (ret_ts, types_ts)
        }
        Err(()) => (quote! { None }, quote! {}),
    };

    let ident = &event_strct.ident;
    let input_generics = &event_strct.generics;
    let (impl_generics, ty_generics, where_clause) = input_generics.split_for_impl();

    quote! {
        impl #impl_generics #ident #ty_generics #where_clause {
            pub fn __anchor_private_gen_idl_event(
                defined_types: &mut std::collections::HashMap<String, #idl::IdlTypeDefinition>,
            ) -> Option<#idl::IdlEvent> {
                #types_ts
                #ret_ts
            }
        }
    }
}

// generates the IDL generation impl for the Accounts struct
pub fn gen_idl_build_impl_for_accounts_struct(
    accs_strct: &AccountsStruct,
    no_docs: bool,
) -> TokenStream {
    let (idl, _) = get_module_paths();

    let ident = &accs_strct.ident;
    let (impl_generics, ty_generics, where_clause) = accs_strct.generics.split_for_impl();

    let (accounts, acc_types): (Vec<TokenStream>, Vec<Option<&syn::TypePath>>) = accs_strct
        .fields
        .iter()
        .map(|acc: &AccountField| match acc {
            AccountField::CompositeField(comp_f) => {
                let ty = if let syn::Type::Path(path) = &comp_f.raw_field.ty {
                    // some::path::Foo<'info> -> some::path::Foo
                    let mut res = syn::Path {
                        leading_colon: path.path.leading_colon,
                        segments: syn::punctuated::Punctuated::new(),
                    };
                    for segment in &path.path.segments {
                        let s = syn::PathSegment {
                            ident: segment.ident.clone(),
                            arguments: syn::PathArguments::None,
                        };
                        res.segments.push(s);
                    };
                    res
                } else {
                    panic!("expecting path")
                };
                let name = comp_f.ident.to_string().to_mixed_case();
                (quote!{
                    #idl::IdlAccountItem::IdlAccounts(#idl::IdlAccounts {
                        name: #name.into(),
                        accounts: <#ty>::__anchor_private_gen_idl_accounts(accounts, defined_types),
                    })
                }, None)
            }
            AccountField::Field(acc) => {
                let name = acc.ident.to_string().to_mixed_case();
                let is_mut = acc.constraints.is_mutable();
                let is_signer = match acc.ty {
                    crate::Ty::Signer => true,
                    _ => acc.constraints.is_signer()
                };
                let is_optional = if acc.is_optional { quote!{Some(true)} } else { quote!{None} };
                let docs = match &acc.docs {
                    Some(docs) if !no_docs => quote! {Some(vec![#(#docs.into()),*])},
                    _ => quote! {None},
                };
                let pda = quote!{None}; // TODO
                let relations = super::parse::relations::parse(acc, get_seeds_feature());

                let acc_type_path = match &acc.ty {
                    crate::Ty::Account(ty) => Some(&ty.account_type_path),
                    crate::Ty::AccountLoader(ty) => Some(&ty.account_type_path),
                    crate::Ty::InterfaceAccount(ty) => Some(&ty.account_type_path),
                    _ => None,
                };

                (quote!{
                    #idl::IdlAccountItem::IdlAccount(#idl::IdlAccount{
                        name: #name.into(),
                        is_mut: #is_mut,
                        is_signer: #is_signer,
                        is_optional: #is_optional,
                        docs: #docs,
                        pda: #pda,
                        relations: vec![#(#relations.into()),*],
                    })
                }, acc_type_path)
            }
        })
        .unzip::<TokenStream, Option<&syn::TypePath>, Vec<TokenStream>, Vec<Option<&syn::TypePath>>>();
    let acc_types = acc_types
        .into_iter()
        .flatten()
        .collect::<Vec<&syn::TypePath>>();

    quote! {
        impl #impl_generics #ident #ty_generics #where_clause {
            pub fn __anchor_private_gen_idl_accounts(
                accounts: &mut std::collections::HashMap<String, #idl::IdlTypeDefinition>,
                defined_types: &mut std::collections::HashMap<String, #idl::IdlTypeDefinition>,
            ) -> Vec<#idl::IdlAccountItem> {
                #({
                    <#acc_types>::__anchor_private_insert_idl_defined(defined_types);

                    let path = <#acc_types>::__anchor_private_full_path();
                    <#acc_types>::__anchor_private_gen_idl_type()
                        .and_then(|ty| accounts.insert(path, ty));

                });*

                vec![#(#accounts),*]
            }
        }
    }
}

// generates the IDL generation print function for the program module
pub fn gen_idl_print_function_for_program(program: &Program, no_docs: bool) -> TokenStream {
    let (idl, serde_json) = get_module_paths();

    let (instructions, defined) = program
        .ixs
        .iter()
        .flat_map(|ix| -> Result<_, ()> {
            let name = ix.ident.to_string().to_mixed_case();
            let docs = match &ix.docs {
                Some(docs) if !no_docs => quote! {Some(vec![#(#docs.into()),*])},
                _ => quote! {None},
            };
            let ctx_ident = &ix.anchor_ident;

            let (args, mut defined) = ix
                .args
                .iter()
                .map(|arg| {
                    let arg_name = arg.name.to_string().to_mixed_case();
                    let docs = match docs::parse(&arg.raw_arg.attrs) {
                        Some(docs) if !no_docs => quote! {Some(vec![#(#docs.into()),*])},
                        _ => quote! {None},
                    };
                    let (ty, defined) = idl_type_ts_from_syn_type(&arg.raw_arg.ty, &vec![])?;

                    Ok((quote! {
                        #idl::IdlField {
                            name: #arg_name.into(),
                            docs: #docs,
                            ty: #ty,
                        }
                    }, defined))
                })
                .collect::<Result<Vec<_>, ()>>()?
                .into_iter()
                .unzip::<TokenStream, Vec<syn::TypePath>, Vec<TokenStream>, Vec<Vec<syn::TypePath>>>();

            let returns = match idl_type_ts_from_syn_type(&ix.returns.ty, &vec![]) {
                Ok((ty, def)) => {
                    defined.push(def);
                    quote!{ Some(#ty) }
                },
                Err(()) => quote!{ None }
            };

            Ok((quote! {
                #idl::IdlInstruction {
                    name: #name.into(),
                    docs: #docs,
                    accounts: #ctx_ident::__anchor_private_gen_idl_accounts(
                        &mut accounts,
                        &mut defined_types,
                    ),
                    args: vec![#(#args),*],
                    returns: #returns,
                }
            }, defined))
        })
        .unzip::<TokenStream, Vec<Vec<syn::TypePath>>, Vec<TokenStream>, Vec<Vec<Vec<syn::TypePath>>>>();
    let defined = defined
        .into_iter()
        .flatten()
        .flatten()
        .collect::<Vec<syn::TypePath>>();

    let name = program.name.to_string();
    let docs = match &program.docs {
        Some(docs) if !no_docs => quote! {Some(vec![#(#docs.into()),*])},
        _ => quote! {None},
    };

    quote! {
        #[test]
        pub fn __anchor_private_print_idl_program() {
            let mut accounts: std::collections::HashMap<String, #idl::IdlTypeDefinition> =
                std::collections::HashMap::new();
            let mut defined_types: std::collections::HashMap<String, #idl::IdlTypeDefinition> =
                std::collections::HashMap::new();

            #({
                <#defined>::__anchor_private_insert_idl_defined(&mut defined_types);

                let path = <#defined>::__anchor_private_full_path();
                <#defined>::__anchor_private_gen_idl_type()
                    .and_then(|ty| defined_types.insert(path, ty));
            });*

            let instructions = vec![#(#instructions),*];

            let idl = #idl::Idl {
                version: env!("CARGO_PKG_VERSION").into(),
                name: #name.into(),
                docs: #docs,
                constants: vec![],
                instructions,
                accounts: accounts.into_values().collect(),
                types: defined_types.into_values().collect(),
                events: None,
                errors: None,
                metadata: None,
            };

            println!("---- IDL begin program ----");
            println!("{}", #serde_json::to_string_pretty(&idl).unwrap());
            println!("---- IDL end program ----");
        }
    }
}

pub fn gen_idl_print_function_for_event(event: &ItemStruct) -> TokenStream {
    let (idl, serde_json) = get_module_paths();

    let ident = &event.ident;
    let fn_name = format_ident!("__anchor_private_print_idl_event_{}", ident.to_string());
    let impl_gen = gen_idl_build_impl_for_event(event);

    quote! {
        #impl_gen

        #[test]
        pub fn #fn_name() {
            let mut defined_types: std::collections::HashMap<String, #idl::IdlTypeDefinition> = std::collections::HashMap::new();
            let event = #ident::__anchor_private_gen_idl_event(&mut defined_types);

            if let Some(event) = event {
                let json = #serde_json::json!({
                    "event": event,
                    "defined_types": defined_types.into_values().collect::<Vec<#idl::IdlTypeDefinition>>()
                });

                println!("---- IDL begin event ----");
                println!("{}", #serde_json::to_string_pretty(&json).unwrap());
                println!("---- IDL end event ----");
            }
        }
    }
}

pub fn gen_idl_print_function_for_constant(item: &syn::ItemConst) -> TokenStream {
    let fn_name = format_ident!(
        "__anchor_private_print_idl_const_{}",
        item.ident.to_string()
    );
    let (idl, serde_json) = get_module_paths();

    let name = item.ident.to_string();
    let expr = &item.expr;

    let impl_ts = match idl_type_ts_from_syn_type(&item.ty, &vec![]) {
        Ok((ty, _)) => quote! {
            let value = format!("{:?}", #expr);

            let idl = #idl::IdlConst {
                name: #name.into(),
                ty: #ty,
                value,
            };

            println!("---- IDL begin const ----");
            println!("{}", #serde_json::to_string_pretty(&idl).unwrap());
            println!("---- IDL end const ----");
        },
        Err(()) => quote! {},
    };

    quote! {
        #[test]
        pub fn #fn_name() {
            #impl_ts
        }
    }
}

pub fn gen_idl_print_function_for_error(error: &Error) -> TokenStream {
    let fn_name = format_ident!(
        "__anchor_private_print_idl_error_{}",
        error.ident.to_string()
    );
    let (idl, serde_json) = get_module_paths();

    let error_codes = error
        .codes
        .iter()
        .map(|code| {
            let id = code.id;
            let name = code.ident.to_string();

            let msg = match code.msg.clone() {
                Some(msg) => quote! { Some(#msg.to_string()) },
                None => quote! { None },
            };

            quote! {
                #idl::IdlErrorCode {
                    code: anchor_lang::error::ERROR_CODE_OFFSET + #id,
                    name: #name.into(),
                    msg: #msg,
                }
            }
        })
        .collect::<Vec<TokenStream>>();

    quote! {
        #[test]
        pub fn #fn_name() {
            let errors = vec![#(#error_codes),*];

            println!("---- IDL begin errors ----");
            println!("{}", #serde_json::to_string_pretty(&errors).unwrap());
            println!("---- IDL end errors ----");
        }
    }
}
