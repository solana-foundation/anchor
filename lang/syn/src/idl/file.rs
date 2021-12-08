use crate::idl::*;
use crate::parser::context::CrateContext;
use crate::parser::{self, accounts, error, program};
use crate::Ty;
use crate::{AccountField, AccountsStruct, StateIx};
use anyhow::Result;
use heck::MixedCase;
use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use std::path::Path;

const DERIVE_NAME: &str = "Accounts";
// TODO: sharee this with `anchor_lang` crate.
const ERROR_CODE_OFFSET: u32 = 6000;

// Parse an entire interface file.
pub fn parse(filename: impl AsRef<Path>, version: String) -> Result<Option<Idl>> {
    let ctx = CrateContext::parse(filename)?;

    let program_mod = match parse_program_mod(&ctx) {
        None => return Ok(None),
        Some(m) => m,
    };
    let p = program::parse(program_mod)?;

    let accs = parse_account_derives(&ctx);

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
                                    accounts,
                                    args,
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
                    let accounts = idl_accounts(accounts_strct, &accs);
                    IdlInstruction {
                        name,
                        accounts,
                        args,
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
    let error = parse_error_enum(&ctx).map(|mut e| error::parse(&mut e, None));
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

    let events = parse_events(&ctx)
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
                    let index = match f.attrs.get(0) {
                        None => false,
                        Some(i) => parser::tts_to_string(&i.path) == "index",
                    };
                    IdlEventField {
                        name: f.ident.clone().unwrap().to_string().to_mixed_case(),
                        ty: parser::tts_to_string(&f.ty).parse().unwrap(),
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
    let ty_defs = parse_ty_defs(&ctx)?;

    let account_structs = parse_accounts(&ctx);
    let account_names: HashSet<String> = account_structs
        .iter()
        .map(|a| a.ident.to_string())
        .collect::<HashSet<_>>();

    let error_name = error.map(|e| e.name).unwrap_or_else(|| "".to_string());

    // All types that aren't in the accounts section, are in the types section.
    for ty_def in ty_defs {
        // Don't add the error type to the types or accounts sections.
        if ty_def.name != error_name {
            if account_names.contains(&ty_def.name) {
                accounts.push(ty_def);
            } else if !events.iter().any(|e| e.name == ty_def.name) {
                types.push(ty_def);
            }
        }
    }

    let constants = parse_consts(&ctx)
        .iter()
        .map(|c: &&syn::ItemConst| IdlConst {
            name: c.ident.to_string(),
            ty: c.ty.to_token_stream().to_string().parse().unwrap(),
            value: c.expr.to_token_stream().to_string().parse().unwrap(),
        })
        .collect::<Vec<IdlConst>>();

    Ok(Some(Idl {
        version,
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
        constants,
    }))
}

// Parse the main program mod.
fn parse_program_mod(ctx: &CrateContext) -> Option<syn::ItemMod> {
    let root = ctx.root_module();
    let mods = root
        .items()
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
        return None;
    }
    Some(mods[0].clone())
}

fn parse_error_enum(ctx: &CrateContext) -> Option<syn::ItemEnum> {
    ctx.enums()
        .filter_map(|item_enum| {
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
        })
        .next()
        .cloned()
}

fn parse_events(ctx: &CrateContext) -> Vec<&syn::ItemStruct> {
    ctx.structs()
        .filter_map(|item_strct| {
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
        })
        .collect()
}

fn parse_accounts(ctx: &CrateContext) -> Vec<&syn::ItemStruct> {
    ctx.structs()
        .filter_map(|item_strct| {
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
        })
        .collect()
}

// Parse all structs implementing the `Accounts` trait.
fn parse_account_derives(ctx: &CrateContext) -> HashMap<String, AccountsStruct> {
    // TODO: parse manual implementations. Currently we only look
    //       for derives.
    ctx.structs()
        .filter_map(|i_strct| {
            for attr in &i_strct.attrs {
                if attr.tokens.to_string().contains(DERIVE_NAME) {
                    let strct = accounts::parse(i_strct).expect("Code not parseable");
                    return Some((strct.ident.to_string(), strct));
                }
            }
            None
        })
        .collect()
}

fn parse_consts(ctx: &CrateContext) -> Vec<&syn::ItemConst> {
    ctx.consts()
        .filter(|item_strct| {
            for attr in &item_strct.attrs {
                if attr.path.segments.last().unwrap().ident == "constant" {
                    return true;
                }
            }
            false
        })
        .collect()
}

// Parse all user defined types in the file.
fn parse_ty_defs(ctx: &CrateContext) -> Result<Vec<IdlTypeDefinition>> {
    ctx.structs()
        .filter_map(|item_strct| {
            // Only take serializable types
            let serializable = item_strct.attrs.iter().any(|attr| {
                let attr_string = attr.tokens.to_string();
                let attr_name = attr.path.segments.last().unwrap().ident.to_string();
                let attr_serializable = ["account", "associated", "event", "zero_copy"];

                let derived_serializable = attr_name == "derive"
                    && attr_string.contains("AnchorSerialize")
                    && attr_string.contains("AnchorDeserialize");

                attr_serializable.iter().any(|a| *a == attr_name) || derived_serializable
            });

            if !serializable {
                return None;
            }

            // Only take public types
            match &item_strct.vis {
                syn::Visibility::Public(_) => (),
                _ => return None,
            }

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
                syn::Fields::Unnamed(_) => return None,
                _ => panic!("Empty structs are allowed."),
            };

            Some(fields.map(|fields| IdlTypeDefinition {
                name,
                ty: IdlTypeDefinitionTy::Struct { fields },
            }))
        })
        .chain(ctx.enums().map(|enm| {
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
            Ok(IdlTypeDefinition {
                name,
                ty: IdlTypeDefinitionTy::Enum { variants },
            })
        }))
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
                is_signer: match acc.ty {
                    Ty::Signer => true,
                    _ => acc.constraints.is_signer(),
                },
            }),
        })
        .collect::<Vec<_>>()
}
