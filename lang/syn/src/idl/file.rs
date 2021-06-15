use crate::idl::*;
use crate::parser::{self, accounts, error, program};
use crate::{AccountField, AccountsStruct, StateIx};
use anyhow::Result;
use heck::MixedCase;
use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::iter::FromIterator;
use std::path::Path;

const DERIVE_NAME: &str = "Accounts";
// TODO: sharee this with `anchor_lang` crate.
const ERROR_CODE_OFFSET: u32 = 300;

// Parse an entire interface file.
pub fn parse(filename: impl AsRef<Path>) -> Result<Idl> {
    let mut file = File::open(&filename)?;

    let mut src = String::new();
    file.read_to_string(&mut src).expect("Unable to read file");

    let f = syn::parse_file(&src).expect("Unable to parse file");

    let p = program::parse(parse_program_mod(&f))?;

    let accs = parse_account_derives(&f);

    let state = match p.state {
        None => None,
        Some(state) => match state.ctor_and_anchor {
            None => None, // State struct defined but no implementation
            Some((ctor, anchor_ident)) => {
                let mut methods = state
                    .impl_block_and_methods
                    .map(|(_impl_block, methods)| {
                        methods
                            .iter()
                            .map(|method: &StateIx| {
                                let name = method.ident.to_string().to_mixed_case();
                                let args = method
                                    .args
                                    .iter()
                                    .map(|arg| {
                                        let mut tts = proc_macro2::TokenStream::new();
                                        arg.raw_arg.ty.to_tokens(&mut tts);
                                        let ty = tts.to_string().parse().unwrap();
                                        IdlField {
                                            name: arg.name.to_string().to_mixed_case(),
                                            ty,
                                        }
                                    })
                                    .collect::<Vec<_>>();
                                let accounts_strct =
                                    accs.get(&method.anchor_ident.to_string()).unwrap();
                                let accounts = idl_accounts(accounts_strct, &accs);
                                IdlInstruction {
                                    name,
                                    args,
                                    accounts,
                                }
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let ctor = {
                    let name = "new".to_string();
                    let args = ctor
                        .sig
                        .inputs
                        .iter()
                        .filter(|arg| match arg {
                            syn::FnArg::Typed(pat_ty) => {
                                // TODO: this filtering should be done in the parser.
                                let mut arg_str = parser::tts_to_string(&pat_ty.ty);
                                arg_str.retain(|c| !c.is_whitespace());
                                !arg_str.starts_with("Context<")
                            }
                            _ => false,
                        })
                        .map(|arg: &syn::FnArg| match arg {
                            syn::FnArg::Typed(arg_typed) => {
                                let mut tts = proc_macro2::TokenStream::new();
                                arg_typed.ty.to_tokens(&mut tts);
                                let ty = tts.to_string().parse().unwrap();
                                IdlField {
                                    name: parser::tts_to_string(&arg_typed.pat).to_mixed_case(),
                                    ty,
                                }
                            }
                            _ => panic!("Invalid syntax"),
                        })
                        .collect();
                    let accounts_strct = accs.get(&anchor_ident.to_string()).unwrap();
                    let accounts = idl_accounts(&accounts_strct, &accs);
                    IdlInstruction {
                        name,
                        args,
                        accounts,
                    }
                };

                methods.insert(0, ctor);

                let strct = {
                    let fields = match state.strct.fields {
                        syn::Fields::Named(f_named) => f_named
                            .named
                            .iter()
                            .map(|f: &syn::Field| {
                                let mut tts = proc_macro2::TokenStream::new();
                                f.ty.to_tokens(&mut tts);
                                let ty = tts.to_string().parse().unwrap();
                                IdlField {
                                    name: f.ident.as_ref().unwrap().to_string().to_mixed_case(),
                                    ty,
                                }
                            })
                            .collect::<Vec<IdlField>>(),
                        _ => panic!("State must be a struct"),
                    };
                    IdlTypeDefinition {
                        name: state.name,
                        ty: IdlTypeDefinitionTy::Struct { fields },
                    }
                };

                Some(IdlState { strct, methods })
            }
        },
    };
    let error = parse_error_enum(&f).map(|mut e| error::parse(&mut e, None));
    let error_codes = error.as_ref().map(|e| {
        e.codes
            .iter()
            .map(|code| IdlErrorCode {
                code: ERROR_CODE_OFFSET + code.id,
                name: code.ident.to_string(),
                msg: code.msg.clone(),
            })
            .collect::<Vec<IdlErrorCode>>()
    });

    let instructions = p
        .ixs
        .iter()
        .map(|ix| {
            let args = ix
                .args
                .iter()
                .map(|arg| {
                    let mut tts = proc_macro2::TokenStream::new();
                    arg.raw_arg.ty.to_tokens(&mut tts);
                    let ty = tts.to_string().parse().unwrap();
                    IdlField {
                        name: arg.name.to_string().to_mixed_case(),
                        ty,
                    }
                })
                .collect::<Vec<_>>();
            // todo: don't unwrap
            let accounts_strct = accs.get(&ix.anchor_ident.to_string()).unwrap();
            let accounts = idl_accounts(accounts_strct, &accs);
            IdlInstruction {
                name: ix.ident.to_string().to_mixed_case(),
                accounts,
                args,
            }
        })
        .collect::<Vec<_>>();

    let events = parse_events(&f)
        .iter()
        .map(|e: &&syn::ItemStruct| {
            let fields = match &e.fields {
                syn::Fields::Named(n) => n,
                _ => panic!("Event fields must be named"),
            };
            let fields = fields
                .named
                .iter()
                .map(|f: &syn::Field| {
                    let index = match f.attrs.iter().next() {
                        None => false,
                        Some(i) => parser::tts_to_string(&i.path) == "index",
                    };
                    IdlEventField {
                        name: f.ident.clone().unwrap().to_string().to_mixed_case(),
                        ty: parser::tts_to_string(&f.ty).to_string().parse().unwrap(),
                        index,
                    }
                })
                .collect::<Vec<IdlEventField>>();

            IdlEvent {
                name: e.ident.to_string(),
                fields,
            }
        })
        .collect::<Vec<IdlEvent>>();

    // All user defined types.
    let mut accounts = vec![];
    let mut types = vec![];
    let ty_defs = parse_ty_defs(&f)?;

    let account_structs = parse_accounts(&f);
    let account_names: HashSet<String> =
        HashSet::from_iter(account_structs.iter().map(|a| a.ident.to_string()));

    let error_name = error.map(|e| e.name).unwrap_or_else(|| "".to_string());

    // All types that aren't in the accounts section, are in the types section.
    for ty_def in ty_defs {
        // Don't add the error type to the types or accounts sections.
        if ty_def.name != error_name {
            if account_names.contains(&ty_def.name) {
                accounts.push(ty_def);
            } else if events.iter().position(|e| e.name == ty_def.name).is_none() {
                types.push(ty_def);
            }
        }
    }

    Ok(Idl {
        version: "0.0.0".to_string(),
        name: p.name.to_string(),
        state,
        instructions,
        types,
        accounts,
        events: if events.is_empty() {
            None
        } else {
            Some(events)
        },
        errors: error_codes,
        metadata: None,
    })
}

// Parse the main program mod.
fn parse_program_mod(f: &syn::File) -> syn::ItemMod {
    let mods = f
        .items
        .iter()
        .filter_map(|i| match i {
            syn::Item::Mod(item_mod) => {
                let mod_count = item_mod
                    .attrs
                    .iter()
                    .filter(|attr| attr.path.segments.last().unwrap().ident == "program")
                    .count();
                if mod_count != 1 {
                    return None;
                }
                Some(item_mod)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if mods.len() != 1 {
        panic!("Did not find program attribute");
    }
    mods[0].clone()
}

fn parse_error_enum(f: &syn::File) -> Option<syn::ItemEnum> {
    f.items
        .iter()
        .filter_map(|i| match i {
            syn::Item::Enum(item_enum) => {
                let attrs_count = item_enum
                    .attrs
                    .iter()
                    .filter(|attr| {
                        let segment = attr.path.segments.last().unwrap();
                        segment.ident == "error"
                    })
                    .count();
                match attrs_count {
                    0 => None,
                    1 => Some(item_enum),
                    _ => panic!("Invalid syntax: one error attribute allowed"),
                }
            }
            _ => None,
        })
        .next()
        .cloned()
}

fn parse_events(f: &syn::File) -> Vec<&syn::ItemStruct> {
    f.items
        .iter()
        .filter_map(|i| match i {
            syn::Item::Struct(item_strct) => {
                let attrs_count = item_strct
                    .attrs
                    .iter()
                    .filter(|attr| {
                        let segment = attr.path.segments.last().unwrap();
                        segment.ident == "event"
                    })
                    .count();
                match attrs_count {
                    0 => None,
                    1 => Some(item_strct),
                    _ => panic!("Invalid syntax: one event attribute allowed"),
                }
            }
            _ => None,
        })
        .collect()
}

fn parse_accounts(f: &syn::File) -> Vec<&syn::ItemStruct> {
    f.items
        .iter()
        .filter_map(|i| match i {
            syn::Item::Struct(item_strct) => {
                let attrs_count = item_strct
                    .attrs
                    .iter()
                    .filter(|attr| {
                        let segment = attr.path.segments.last().unwrap();
                        segment.ident == "account" || segment.ident == "associated"
                    })
                    .count();
                match attrs_count {
                    0 => None,
                    1 => Some(item_strct),
                    _ => panic!("Invalid syntax: one event attribute allowed"),
                }
            }
            _ => None,
        })
        .collect()
}

// Parse all structs implementing the `Accounts` trait.
fn parse_account_derives(f: &syn::File) -> HashMap<String, AccountsStruct> {
    f.items
        .iter()
        .filter_map(|i| match i {
            syn::Item::Struct(i_strct) => {
                for attr in &i_strct.attrs {
                    if attr.tokens.to_string().contains(DERIVE_NAME) {
                        let strct = accounts::parse(i_strct).expect("Code not parseable");
                        return Some((strct.ident.to_string(), strct));
                    }
                }
                None
            }
            // TODO: parse manual implementations. Currently we only look
            //       for derives.
            _ => None,
        })
        .collect()
}

// Parse all user defined types in the file.
fn parse_ty_defs(f: &syn::File) -> Result<Vec<IdlTypeDefinition>> {
    f.items
        .iter()
        .filter_map(|i| match i {
            syn::Item::Struct(item_strct) => {
                for attr in &item_strct.attrs {
                    if attr.tokens.to_string().contains(DERIVE_NAME) {
                        return None;
                    }
                }
                if let syn::Visibility::Public(_) = &item_strct.vis {
                    let name = item_strct.ident.to_string();
                    let fields = match &item_strct.fields {
                        syn::Fields::Named(fields) => fields
                            .named
                            .iter()
                            .map(|f: &syn::Field| {
                                let mut tts = proc_macro2::TokenStream::new();
                                f.ty.to_tokens(&mut tts);
                                Ok(IdlField {
                                    name: f.ident.as_ref().unwrap().to_string().to_mixed_case(),
                                    ty: tts.to_string().parse()?,
                                })
                            })
                            .collect::<Result<Vec<IdlField>>>(),
                        _ => panic!("Only named structs are allowed."),
                    };

                    return Some(fields.map(|fields| IdlTypeDefinition {
                        name,
                        ty: IdlTypeDefinitionTy::Struct { fields },
                    }));
                }
                None
            }
            syn::Item::Enum(enm) => {
                let name = enm.ident.to_string();
                let variants = enm
                    .variants
                    .iter()
                    .map(|variant: &syn::Variant| {
                        let name = variant.ident.to_string();
                        let fields = match &variant.fields {
                            syn::Fields::Unit => None,
                            syn::Fields::Unnamed(fields) => {
                                let fields: Vec<IdlType> =
                                    fields.unnamed.iter().map(to_idl_type).collect();
                                Some(EnumFields::Tuple(fields))
                            }
                            syn::Fields::Named(fields) => {
                                let fields: Vec<IdlField> = fields
                                    .named
                                    .iter()
                                    .map(|f: &syn::Field| {
                                        let name = f.ident.as_ref().unwrap().to_string();
                                        let ty = to_idl_type(f);
                                        IdlField { name, ty }
                                    })
                                    .collect();
                                Some(EnumFields::Named(fields))
                            }
                        };
                        IdlEnumVariant { name, fields }
                    })
                    .collect::<Vec<IdlEnumVariant>>();
                Some(Ok(IdlTypeDefinition {
                    name,
                    ty: IdlTypeDefinitionTy::Enum { variants },
                }))
            }
            _ => None,
        })
        .collect()
}

fn to_idl_type(f: &syn::Field) -> IdlType {
    let mut tts = proc_macro2::TokenStream::new();
    f.ty.to_tokens(&mut tts);
    tts.to_string().parse().unwrap()
}

fn idl_accounts(
    accounts: &AccountsStruct,
    global_accs: &HashMap<String, AccountsStruct>,
) -> Vec<IdlAccountItem> {
    accounts
        .fields
        .iter()
        .map(|acc: &AccountField| match acc {
            AccountField::CompositeField(comp_f) => {
                let accs_strct = global_accs
                    .get(&comp_f.symbol)
                    .expect("Could not resolve Accounts symbol");
                let accounts = idl_accounts(accs_strct, global_accs);
                IdlAccountItem::IdlAccounts(IdlAccounts {
                    name: comp_f.ident.to_string().to_mixed_case(),
                    accounts,
                })
            }
            AccountField::Field(acc) => IdlAccountItem::IdlAccount(IdlAccount {
                name: acc.ident.to_string().to_mixed_case(),
                is_mut: acc.constraints.is_mutable(),
                is_signer: acc.constraints.is_signer(),
            }),
        })
        .collect::<Vec<_>>()
}
