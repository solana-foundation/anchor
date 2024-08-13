use anyhow::{anyhow, Result};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

use super::common::{get_idl_module_path, get_no_docs};
use crate::{AccountField, AccountsStruct, Field, InitKind, Ty};

/// Generate the IDL build impl for the Accounts struct.
pub fn gen_idl_build_impl_accounts_struct(accounts: &AccountsStruct) -> TokenStream {
    let resolution = option_env!("ANCHOR_IDL_BUILD_RESOLUTION")
        .map(|val| val == "TRUE")
        .unwrap_or_default();
    let no_docs = get_no_docs();
    let idl = get_idl_module_path();

    let ident = &accounts.ident;
    let (impl_generics, ty_generics, where_clause) = accounts.generics.split_for_impl();

    let (accounts, defined) = accounts
        .fields
        .iter()
        .map(|acc| match acc {
            AccountField::Field(acc) => {
                let name = acc.ident.to_string();
                let writable = acc.constraints.is_mutable();
                let signer = match acc.ty {
                    Ty::Signer => true,
                    _ => acc.constraints.is_signer(),
                };
                let optional = acc.is_optional;
                let docs = match &acc.docs {
                    Some(docs) if !no_docs => quote! { vec![#(#docs.into()),*] },
                    _ => quote! { vec![] },
                };

                let (address, pda, relations) = if resolution {
                    (
                        get_address(acc),
                        get_pda(acc, accounts),
                        get_relations(acc, accounts),
                    )
                } else {
                    (quote! { None }, quote! { None }, quote! { vec![] })
                };

                let acc_type_path = match &acc.ty {
                    Ty::Account(ty)
                    // Skip `UpgradeableLoaderState` type for now until `bincode` serialization
                    // is supported.
                    //
                    // TODO: Remove this once either `bincode` serialization is supported or
                    // we wrap the type in order to implement `IdlBuild` in `anchor-lang`.
                        if !ty
                            .account_type_path
                            .path
                            .to_token_stream()
                            .to_string()
                            .contains("UpgradeableLoaderState") =>
                    {
                        Some(&ty.account_type_path)
                    }
                    Ty::AccountLoader(ty) => Some(&ty.account_type_path),
                    Ty::InterfaceAccount(ty) => Some(&ty.account_type_path),
                    _ => None,
                };

                (
                    quote! {
                        #idl::IdlInstructionAccountItem::Single(#idl::IdlInstructionAccount {
                            name: #name.into(),
                            docs: #docs,
                            writable: #writable,
                            signer: #signer,
                            optional: #optional,
                            address: #address,
                            pda: #pda,
                            relations: #relations,
                        })
                    },
                    acc_type_path,
                )
            }
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
                    }
                    res
                } else {
                    panic!(
                        "Compose field type must be a path but received: {:?}",
                        comp_f.raw_field.ty
                    )
                };
                let name = comp_f.ident.to_string();

                (
                    quote! {
                        #idl::IdlInstructionAccountItem::Composite(#idl::IdlInstructionAccounts {
                            name: #name.into(),
                            accounts: <#ty>::__anchor_private_gen_idl_accounts(accounts, types),
                        })
                    },
                    None,
                )
            }
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();
    let defined = defined.into_iter().flatten().collect::<Vec<_>>();

    quote! {
        impl #impl_generics #ident #ty_generics #where_clause {
            pub fn __anchor_private_gen_idl_accounts(
                accounts: &mut std::collections::BTreeMap<String, #idl::IdlAccount>,
                types: &mut std::collections::BTreeMap<String, #idl::IdlTypeDef>,
            ) -> Vec<#idl::IdlInstructionAccountItem> {
                #(
                    if let Some(ty) = <#defined>::create_type() {
                        let account = #idl::IdlAccount {
                            name: ty.name.clone(),
                            discriminator: #defined::DISCRIMINATOR.into(),
                        };
                        accounts.insert(account.name.clone(), account);
                        types.insert(ty.name.clone(), ty);
                        <#defined>::insert_types(types);
                    }
                );*

                vec![#(#accounts),*]
            }
        }
    }
}

fn get_address(acc: &Field) -> TokenStream {
    match &acc.ty {
        Ty::Program(ty) => ty
            .account_type_path
            .path
            .segments
            .last()
            .map(|seg| &seg.ident)
            .map(|ident| quote! { Some(#ident::id().to_string()) })
            .unwrap_or_else(|| quote! { None }),
        Ty::Sysvar(_) => {
            let ty = acc.account_ty();
            let sysvar_id_trait = quote!(anchor_lang::solana_program::sysvar::SysvarId);
            quote! { Some(<#ty as #sysvar_id_trait>::id().to_string()) }
        }
        _ => acc
            .constraints
            .address
            .as_ref()
            .map(|constraint| &constraint.address)
            .filter(|address| !matches!(address, syn::Expr::Field(_)))
            .map(|address| quote! { Some(#address.to_string()) })
            .unwrap_or_else(|| quote! { None }),
    }
}

fn get_pda(acc: &Field, accounts: &AccountsStruct) -> TokenStream {
    let idl = get_idl_module_path();
    let parse_default = |expr: &syn::Expr| parse_seed(expr, accounts);

    // Seeds
    let seed_constraints = acc.constraints.seeds.as_ref();
    let pda = seed_constraints
        .map(|seed| seed.seeds.iter().map(parse_default))
        .and_then(|seeds| seeds.collect::<Result<Vec<_>>>().ok())
        .map(|seeds| {
            let program = seed_constraints
                .and_then(|seed| seed.program_seed.as_ref())
                .and_then(|program| parse_default(program).ok())
                .map(|program| quote! { Some(#program) })
                .unwrap_or_else(|| quote! { None });

            quote! {
                Some(
                    #idl::IdlPda {
                        seeds: vec![#(#seeds),*],
                        program: #program,
                    }
                )
            }
        });
    if let Some(pda) = pda {
        return pda;
    }

    // Associated token
    let pda = acc
        .constraints
        .init
        .as_ref()
        .and_then(|init| match &init.kind {
            InitKind::AssociatedToken {
                owner,
                mint,
                token_program,
            } => Some((owner, mint, token_program)),
            _ => None,
        })
        .or_else(|| {
            acc.constraints
                .associated_token
                .as_ref()
                .map(|ata| (&ata.wallet, &ata.mint, &ata.token_program))
        })
        .and_then(|(wallet, mint, token_program)| {
            // ATA constraints have implicit `.key()` call
            let parse_expr = |ts| parse_default(&syn::parse2(ts).unwrap()).ok();
            let parse_ata = |expr| parse_expr(quote! { #expr.key().as_ref() });

            let wallet = parse_ata(wallet);
            let mint = parse_ata(mint);
            let token_program = token_program
                .as_ref()
                .and_then(parse_ata)
                .or_else(|| parse_expr(quote!(anchor_spl::token::ID)));

            let seeds = match (wallet, mint, token_program) {
                (Some(w), Some(m), Some(tp)) => quote! { vec![#w, #tp, #m] },
                _ => return None,
            };

            let program = parse_expr(quote!(anchor_spl::associated_token::ID))
                .map(|program| quote! { Some(#program) })
                .unwrap();

            Some(quote! {
                Some(
                    #idl::IdlPda {
                        seeds: #seeds,
                        program: #program,
                    }
                )
            })
        });
    if let Some(pda) = pda {
        return pda;
    }

    quote! { None }
}

/// Parse a seeds constraint, extracting the `IdlSeed` types.
///
/// Note: This implementation makes assumptions about the types that can be used (e.g., no
/// program-defined function calls in seeds).
///
/// This probably doesn't cover all cases. If you see a warning log, you can add a new case here.
/// In the worst case, we miss a seed and the parser will treat the given seeds as empty and so
/// clients will simply fail to automatically populate the PDA accounts.
///
/// # Seed assumptions
///
/// Seeds must be of one of the following forms:
///
/// - Constant
/// - Instruction argument
/// - Account key or field
fn parse_seed(seed: &syn::Expr, accounts: &AccountsStruct) -> Result<TokenStream> {
    let idl = get_idl_module_path();
    let args = accounts.instruction_args().unwrap_or_default();
    match seed {
        syn::Expr::MethodCall(_) => {
            let seed_path = SeedPath::new(seed)?;

            if args.contains_key(&seed_path.name) {
                let path = seed_path.path();

                Ok(quote! {
                    #idl::IdlSeed::Arg(
                        #idl::IdlSeedArg {
                            path: #path.into(),
                        }
                    )
                })
            } else if let Some(account_field) = accounts
                .fields
                .iter()
                .find(|field| *field.ident() == seed_path.name)
            {
                let path = seed_path.path();
                let account = match account_field.ty_name() {
                    Some(name) if !seed_path.subfields.is_empty() => {
                        quote! { Some(#name.into()) }
                    }
                    _ => quote! { None },
                };

                Ok(quote! {
                    #idl::IdlSeed::Account(
                        #idl::IdlSeedAccount {
                            path: #path.into(),
                            account: #account,
                        }
                    )
                })
            } else if seed_path.name.contains('"') {
                let seed = seed_path.name.trim_start_matches("b\"").trim_matches('"');
                Ok(quote! {
                    #idl::IdlSeed::Const(
                        #idl::IdlSeedConst {
                            value: #seed.into(),
                        }
                    )
                })
            } else {
                Ok(quote! {
                    #idl::IdlSeed::Const(
                        #idl::IdlSeedConst {
                            value: #seed.into(),
                        }
                    )
                })
            }
        }
        syn::Expr::Path(path) => {
            let seed = path
                .path
                .get_ident()
                .map(|ident| ident.to_string())
                .filter(|ident| args.contains_key(ident))
                .map(|path| {
                    quote! {
                        #idl::IdlSeed::Arg(
                            #idl::IdlSeedArg {
                                path: #path.into(),
                            }
                        )
                    }
                })
                .unwrap_or_else(|| {
                    // Not all types can be converted to `Vec<u8>` with `.into` call e.g. `Pubkey`.
                    // This is problematic for `seeds::program` but a hacky way to handle this
                    // scenerio is to check whether the last segment of the path ends with `ID`.
                    let seed = path
                        .path
                        .segments
                        .last()
                        .filter(|seg| seg.ident.to_string().ends_with("ID"))
                        .map(|_| quote! { #seed.as_ref() })
                        .unwrap_or_else(|| quote! { #seed });
                    quote! {
                        #idl::IdlSeed::Const(
                            #idl::IdlSeedConst {
                                value: #seed.into(),
                            }
                        )
                    }
                });
            Ok(seed)
        }
        syn::Expr::Lit(_) => Ok(quote! {
            #idl::IdlSeed::Const(
                #idl::IdlSeedConst {
                    value: #seed.into(),
                }
            )
        }),
        syn::Expr::Reference(rf) => parse_seed(&rf.expr, accounts),
        _ => Err(anyhow!("Unexpected seed: {seed:?}")),
    }
}

/// SeedPath represents the deconstructed syntax of a single pda seed,
/// consisting of a variable name and a vec of all the sub fields accessed
/// on that variable name. For example, if a seed is `my_field.my_data.as_ref()`,
/// then the field name is `my_field` and the vec of sub fields is `[my_data]`.
struct SeedPath {
    /// Seed name
    name: String,
    /// All path components for the subfields accessed on this seed
    subfields: Vec<String>,
}

impl SeedPath {
    /// Extract the seed path from a single seed expression.
    fn new(seed: &syn::Expr) -> Result<Self> {
        // Convert the seed into the raw string representation.
        let seed_str = seed.to_token_stream().to_string();

        // Check unsupported cases e.g. `&(account.field + 1).to_le_bytes()`
        if !seed_str.contains('"')
            && seed_str.contains(|c: char| matches!(c, '+' | '-' | '*' | '/' | '%' | '^'))
        {
            return Err(anyhow!("Seed expression not supported: {seed:#?}"));
        }

        // Break up the seed into each subfield component.
        let mut components = seed_str.split('.').collect::<Vec<_>>();
        if components.len() <= 1 {
            return Err(anyhow!("Seed is in unexpected format: {seed:#?}"));
        }

        // The name of the variable (or field).
        let name = components.remove(0).to_owned();

        // The path to the seed (only if the `name` type is a struct).
        let mut path = Vec::new();
        while !components.is_empty() {
            let subfield = components.remove(0);
            if subfield.contains("()") {
                break;
            }
            path.push(subfield.into());
        }
        if path.len() == 1 && (path[0] == "key" || path[0] == "key()") {
            path = Vec::new();
        }

        Ok(SeedPath {
            name,
            subfields: path,
        })
    }

    /// Get the full path to the data this seed represents.
    fn path(&self) -> String {
        match self.subfields.len() {
            0 => self.name.to_owned(),
            _ => format!("{}.{}", self.name, self.subfields.join(".")),
        }
    }
}

fn get_relations(acc: &Field, accounts: &AccountsStruct) -> TokenStream {
    let relations = accounts
        .fields
        .iter()
        .filter_map(|af| match af {
            AccountField::Field(f) => f
                .constraints
                .has_one
                .iter()
                .filter_map(|c| match &c.join_target {
                    syn::Expr::Path(path) => path
                        .path
                        .segments
                        .first()
                        .filter(|seg| seg.ident == acc.ident)
                        .map(|_| Some(f.ident.to_string())),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>(),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    quote! { vec![#(#relations.into()),*] }
}
