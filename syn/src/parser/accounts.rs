use crate::{
    AccountField, AccountsStruct, CompositeField, Constraint, ConstraintBelongsTo,
    ConstraintLiteral, ConstraintOwner, ConstraintRentExempt, ConstraintSeeds, ConstraintSigner,
    CpiAccountTy, Field, ProgramAccountTy, ProgramStateTy, SysvarTy, Ty,
};

pub fn parse(strct: &syn::ItemStruct) -> AccountsStruct {
    let fields = match &strct.fields {
        syn::Fields::Named(fields) => fields,
        _ => panic!("invalid input"),
    };

    let fields: Vec<AccountField> = fields
        .named
        .iter()
        .map(|f: &syn::Field| {
            let anchor_attr = {
                let anchor_attrs: Vec<&syn::Attribute> = f
                    .attrs
                    .iter()
                    .filter_map(|attr: &syn::Attribute| {
                        if attr.path.segments.len() != 1 {
                            return None;
                        }
                        if attr.path.segments[0].ident.to_string() != "account" {
                            return None;
                        }
                        Some(attr)
                    })
                    .collect();
                match anchor_attrs.len() {
                    0 => None,
                    1 => Some(anchor_attrs[0]),
                    _ => panic!("invalid syntax: only one account attribute is allowed"),
                }
            };
            parse_field(f, anchor_attr)
        })
        .collect();

    AccountsStruct::new(strct.clone(), fields)
}

fn parse_field(f: &syn::Field, anchor: Option<&syn::Attribute>) -> AccountField {
    let ident = f.ident.clone().unwrap();
    let (constraints, is_mut, is_signer, is_init) = match anchor {
        None => (vec![], false, false, false),
        Some(anchor) => parse_constraints(anchor),
    };
    match is_field_primitive(f) {
        true => {
            let ty = parse_ty(f);
            AccountField::Field(Field {
                ident,
                ty,
                constraints,
                is_mut,
                is_signer,
                is_init,
            })
        }
        false => AccountField::AccountsStruct(CompositeField {
            ident,
            symbol: ident_string(f),
            constraints,
            raw_field: f.clone(),
        }),
    }
}

fn is_field_primitive(f: &syn::Field) -> bool {
    match ident_string(f).as_str() {
        "ProgramState" | "ProgramAccount" | "CpiAccount" | "Sysvar" | "AccountInfo" => true,
        _ => false,
    }
}

fn parse_ty(f: &syn::Field) -> Ty {
    let path = match &f.ty {
        syn::Type::Path(ty_path) => ty_path.path.clone(),
        _ => panic!("invalid account syntax"),
    };
    match ident_string(f).as_str() {
        "ProgramState" => Ty::ProgramState(parse_program_state(&path)),
        "ProgramAccount" => Ty::ProgramAccount(parse_program_account(&path)),
        "CpiAccount" => Ty::CpiAccount(parse_cpi_account(&path)),
        "Sysvar" => Ty::Sysvar(parse_sysvar(&path)),
        "AccountInfo" => Ty::AccountInfo,
        _ => panic!("invalid account type"),
    }
}

fn ident_string(f: &syn::Field) -> String {
    let path = match &f.ty {
        syn::Type::Path(ty_path) => ty_path.path.clone(),
        _ => panic!("invalid account syntax"),
    };
    // TODO: allow segmented paths.
    assert!(path.segments.len() == 1);
    let segments = &path.segments[0];
    segments.ident.to_string()
}

fn parse_program_state(path: &syn::Path) -> ProgramStateTy {
    let account_ident = parse_account(&path);
    ProgramStateTy { account_ident }
}

fn parse_cpi_account(path: &syn::Path) -> CpiAccountTy {
    let account_ident = parse_account(path);
    CpiAccountTy { account_ident }
}

fn parse_program_account(path: &syn::Path) -> ProgramAccountTy {
    let account_ident = parse_account(path);
    ProgramAccountTy { account_ident }
}

fn parse_account(path: &syn::Path) -> syn::Ident {
    let segments = &path.segments[0];
    let account_ident = match &segments.arguments {
        syn::PathArguments::AngleBracketed(args) => {
            // Expected: <'info, MyType>.
            assert!(args.args.len() == 2);
            match &args.args[1] {
                syn::GenericArgument::Type(ty) => match ty {
                    syn::Type::Path(ty_path) => {
                        // TODO: allow segmented paths.
                        assert!(ty_path.path.segments.len() == 1);
                        let path_segment = &ty_path.path.segments[0];
                        path_segment.ident.clone()
                    }
                    _ => panic!("Invalid ProgramAccount"),
                },
                _ => panic!("Invalid ProgramAccount"),
            }
        }
        _ => panic!("Invalid ProgramAccount"),
    };
    account_ident
}

fn parse_sysvar(path: &syn::Path) -> SysvarTy {
    let segments = &path.segments[0];
    let account_ident = match &segments.arguments {
        syn::PathArguments::AngleBracketed(args) => {
            // Expected: <'info, MyType>.
            assert!(args.args.len() == 2);
            match &args.args[1] {
                syn::GenericArgument::Type(ty) => match ty {
                    syn::Type::Path(ty_path) => {
                        // TODO: allow segmented paths.
                        assert!(ty_path.path.segments.len() == 1);
                        let path_segment = &ty_path.path.segments[0];
                        path_segment.ident.clone()
                    }
                    _ => panic!("Invalid Sysvar"),
                },
                _ => panic!("Invalid Sysvar"),
            }
        }
        _ => panic!("Invalid Sysvar"),
    };
    match account_ident.to_string().as_str() {
        "Clock" => SysvarTy::Clock,
        "Rent" => SysvarTy::Rent,
        "EpochSchedule" => SysvarTy::EpochSchedule,
        "Fees" => SysvarTy::Fees,
        "RecentBlockhashes" => SysvarTy::RecentBlockHashes,
        "SlotHashes" => SysvarTy::SlotHashes,
        "SlotHistory" => SysvarTy::SlotHistory,
        "StakeHistory" => SysvarTy::StakeHistory,
        "Instructions" => SysvarTy::Instructions,
        "Rewards" => SysvarTy::Rewards,
        _ => panic!("Invalid Sysvar"),
    }
}

fn parse_constraints(anchor: &syn::Attribute) -> (Vec<Constraint>, bool, bool, bool) {
    let mut tts = anchor.tokens.clone().into_iter();
    let g_stream = match tts.next().expect("Must have a token group") {
        proc_macro2::TokenTree::Group(g) => g.stream(),
        _ => panic!("Invalid syntax"),
    };

    let mut is_init = false;
    let mut is_mut = false;
    let mut is_signer = false;
    let mut constraints = vec![];
    let mut is_rent_exempt = None;

    let mut inner_tts = g_stream.into_iter();
    while let Some(token) = inner_tts.next() {
        match token {
            proc_macro2::TokenTree::Ident(ident) => match ident.to_string().as_str() {
                "init" => {
                    is_init = true;
                    is_mut = true;
                    // If it's not specified, all program owned accounts default
                    // to being rent exempt.
                    if is_rent_exempt.is_none() {
                        is_rent_exempt = Some(true);
                    }
                }
                "mut" => {
                    is_mut = true;
                }
                "signer" => {
                    is_signer = true;
                    constraints.push(Constraint::Signer(ConstraintSigner {}));
                }
                "seeds" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    let seeds = match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Group(g) => g,
                        _ => panic!("invalid syntax"),
                    };
                    constraints.push(Constraint::Seeds(ConstraintSeeds { seeds }))
                }
                "belongs_to" | "has_one" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    let join_target = match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Ident(ident) => ident,
                        _ => panic!("invalid syntax"),
                    };
                    constraints.push(Constraint::BelongsTo(ConstraintBelongsTo { join_target }))
                }
                "owner" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    let owner = match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Ident(ident) => ident,
                        _ => panic!("invalid syntax"),
                    };
                    let constraint = match owner.to_string().as_str() {
                        "program" => ConstraintOwner::Program,
                        "skip" => ConstraintOwner::Skip,
                        _ => panic!("invalid syntax"),
                    };
                    constraints.push(Constraint::Owner(constraint));
                }
                "rent_exempt" => {
                    match inner_tts.next() {
                        None => is_rent_exempt = Some(true),
                        Some(tkn) => {
                            match tkn {
                                proc_macro2::TokenTree::Punct(punct) => {
                                    assert!(punct.as_char() == '=');
                                    punct
                                }
                                _ => panic!("invalid syntax"),
                            };
                            let should_skip = match inner_tts.next().unwrap() {
                                proc_macro2::TokenTree::Ident(ident) => ident,
                                _ => panic!("invalid syntax"),
                            };
                            match should_skip.to_string().as_str() {
                                "skip" => {
                                    is_rent_exempt = Some(false);
                                },
                                _ => panic!("invalid syntax: omit the rent_exempt attribute to enforce rent exemption"),
                            };
                        }
                    };
                }
                _ => {
                    panic!("invalid syntax");
                }
            },
            proc_macro2::TokenTree::Punct(punct) => {
                if punct.as_char() != ',' {
                    panic!("invalid syntax");
                }
            }
            proc_macro2::TokenTree::Literal(literal) => {
                let tokens: proc_macro2::TokenStream =
                    literal.to_string().replace("\"", "").parse().unwrap();
                constraints.push(Constraint::Literal(ConstraintLiteral { tokens }));
            }
            _ => {
                panic!("invalid syntax");
            }
        }
    }

    if let Some(is_re) = is_rent_exempt {
        match is_re {
            false => constraints.push(Constraint::RentExempt(ConstraintRentExempt::Skip)),
            true => constraints.push(Constraint::RentExempt(ConstraintRentExempt::Enforce)),
        }
    }

    (constraints, is_mut, is_signer, is_init)
}
