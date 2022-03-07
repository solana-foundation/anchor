use crate::*;
use syn::parse::{Error as ParseError, Result as ParseResult};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::Expr;

pub mod constraints;

pub fn parse(strct: &syn::ItemStruct) -> ParseResult<AccountsStruct> {
    let instruction_api: Option<Punctuated<Expr, Comma>> = strct
        .attrs
        .iter()
        .find(|a| {
            a.path
                .get_ident()
                .map_or(false, |ident| ident == "instruction")
        })
        .map(|ix_attr| ix_attr.parse_args_with(Punctuated::<Expr, Comma>::parse_terminated))
        .transpose()?;
    let fields = match &strct.fields {
        syn::Fields::Named(fields) => fields
            .named
            .iter()
            .map(|f| parse_account_field(f, instruction_api.is_some()))
            .collect::<ParseResult<Vec<AccountField>>>()?,
        _ => {
            return Err(ParseError::new_spanned(
                &strct.fields,
                "fields must be named",
            ))
        }
    };

    let _ = constraints_cross_checks(&fields)?;

    Ok(AccountsStruct::new(strct.clone(), fields, instruction_api))
}

fn constraints_cross_checks(fields: &[AccountField]) -> ParseResult<()> {
    // INIT
    let init_fields: Vec<&Field> = fields
        .iter()
        .filter_map(|f| match f {
            AccountField::Field(field) if field.constraints.init.is_some() => Some(field),
            _ => None,
        })
        .collect();

    if !init_fields.is_empty() {
        // init needs system program.
        if fields.iter().all(|f| f.ident() != "system_program") {
            return Err(ParseError::new(
                init_fields[0].ident.span(),
                "the init constraint requires \
                the system_program field to exist in the account \
                validation struct. Use the program type to add \
                the system_program field to your validation struct.",
            ));
        }

        let kind = &init_fields[0].constraints.init.as_ref().unwrap().kind;
        // init token/a_token/mint needs token program.
        match kind {
            InitKind::Program { .. } => (),
            InitKind::Token { .. } | InitKind::AssociatedToken { .. } | InitKind::Mint { .. } => {
                if fields.iter().all(|f| f.ident() != "token_program") {
                    return Err(ParseError::new(
                        init_fields[0].ident.span(),
                        "the init constraint requires \
                            the token_program field to exist in the account \
                            validation struct. Use the program type to add \
                            the token_program field to your validation struct.",
                    ));
                }
            }
        }
        // a_token needs associated token program.
        if let InitKind::AssociatedToken { .. } = kind {
            if fields
                .iter()
                .all(|f| f.ident() != "associated_token_program")
            {
                return Err(ParseError::new(
                    init_fields[0].ident.span(),
                    "the init constraint requires \
                    the associated_token_program field to exist in the account \
                    validation struct. Use the program type to add \
                    the associated_token_program field to your validation struct.",
                ));
            }
        }

        for field in init_fields {
            // Get payer for init-ed account
            let associated_payer_name = match field.constraints.init.clone().unwrap().payer {
                // composite payer, check not supported
                Expr::Field(_) => continue,
                field_name => field_name.to_token_stream().to_string(),
            };

            // Check payer is mutable
            let associated_payer_field = fields.iter().find_map(|f| match f {
                AccountField::Field(field) if *f.ident() == associated_payer_name => Some(field),
                _ => None,
            });
            match associated_payer_field {
                Some(associated_payer_field) => {
                    if !associated_payer_field.constraints.is_mutable() {
                        return Err(ParseError::new(
                            field.ident.span(),
                            "the payer specified for an init constraint must be mutable.",
                        ));
                    }
                }
                _ => {
                    return Err(ParseError::new(
                        field.ident.span(),
                        "the payer specified does not exist.",
                    ));
                }
            }
        }
    }
    Ok(())
}

pub fn parse_account_field(f: &syn::Field, has_instruction_api: bool) -> ParseResult<AccountField> {
    let ident = f.ident.clone().unwrap();
    let docs: String = f
        .attrs
        .iter()
        .map(|a| {
            let meta_result = a.parse_meta();
            if let Ok(syn::Meta::NameValue(meta)) = meta_result {
                if meta.path.is_ident("doc") {
                    if let syn::Lit::Str(doc) = meta.lit {
                        return format!(" {}\n", doc.value().trim());
                    }
                }
            }
            "".to_string()
        })
        .collect::<String>();
    let account_field = match is_field_primitive(f)? {
        true => {
            let ty = parse_ty(f)?;
            let (account_constraints, instruction_constraints) =
                constraints::parse(f, Some(&ty), has_instruction_api)?;
            AccountField::Field(Field {
                ident,
                ty,
                constraints: account_constraints,
                instruction_constraints,
                docs,
            })
        }
        false => {
            let (account_constraints, instruction_constraints) =
                constraints::parse(f, None, has_instruction_api)?;
            AccountField::CompositeField(CompositeField {
                ident,
                constraints: account_constraints,
                instruction_constraints,
                symbol: ident_string(f)?,
                raw_field: f.clone(),
                docs,
            })
        }
    };
    Ok(account_field)
}

fn is_field_primitive(f: &syn::Field) -> ParseResult<bool> {
    let r = matches!(
        ident_string(f)?.as_str(),
        "ProgramState"
            | "ProgramAccount"
            | "CpiAccount"
            | "Sysvar"
            | "AccountInfo"
            | "UncheckedAccount"
            | "CpiState"
            | "Loader"
            | "AccountLoader"
            | "Account"
            | "Program"
            | "Signer"
            | "SystemAccount"
            | "ProgramData"
    );
    Ok(r)
}

fn parse_ty(f: &syn::Field) -> ParseResult<Ty> {
    let path = match &f.ty {
        syn::Type::Path(ty_path) => ty_path.path.clone(),
        _ => return Err(ParseError::new(f.ty.span(), "invalid account type given")),
    };
    let ty = match ident_string(f)?.as_str() {
        "ProgramState" => Ty::ProgramState(parse_program_state(&path)?),
        "CpiState" => Ty::CpiState(parse_cpi_state(&path)?),
        "ProgramAccount" => Ty::ProgramAccount(parse_program_account(&path)?),
        "CpiAccount" => Ty::CpiAccount(parse_cpi_account(&path)?),
        "Sysvar" => Ty::Sysvar(parse_sysvar(&path)?),
        "AccountInfo" => Ty::AccountInfo,
        "UncheckedAccount" => Ty::UncheckedAccount,
        "Loader" => Ty::Loader(parse_program_account_zero_copy(&path)?),
        "AccountLoader" => Ty::AccountLoader(parse_program_account_loader(&path)?),
        "Account" => Ty::Account(parse_account_ty(&path)?),
        "Program" => Ty::Program(parse_program_ty(&path)?),
        "Signer" => Ty::Signer,
        "SystemAccount" => Ty::SystemAccount,
        "ProgramData" => Ty::ProgramData,
        _ => return Err(ParseError::new(f.ty.span(), "invalid account type given")),
    };

    Ok(ty)
}

fn ident_string(f: &syn::Field) -> ParseResult<String> {
    let path = match &f.ty {
        syn::Type::Path(ty_path) => ty_path.path.clone(),
        _ => return Err(ParseError::new(f.ty.span(), "invalid type")),
    };
    if parser::tts_to_string(&path)
        .replace(' ', "")
        .starts_with("Box<Account<")
    {
        return Ok("Account".to_string());
    }
    // TODO: allow segmented paths.
    if path.segments.len() != 1 {
        return Err(ParseError::new(
            f.ty.span(),
            "segmented paths are not currently allowed",
        ));
    }

    let segments = &path.segments[0];
    Ok(segments.ident.to_string())
}

fn parse_program_state(path: &syn::Path) -> ParseResult<ProgramStateTy> {
    let account_ident = parse_account(path)?;
    Ok(ProgramStateTy {
        account_type_path: account_ident,
    })
}

fn parse_cpi_state(path: &syn::Path) -> ParseResult<CpiStateTy> {
    let account_ident = parse_account(path)?;
    Ok(CpiStateTy {
        account_type_path: account_ident,
    })
}

fn parse_cpi_account(path: &syn::Path) -> ParseResult<CpiAccountTy> {
    let account_ident = parse_account(path)?;
    Ok(CpiAccountTy {
        account_type_path: account_ident,
    })
}

fn parse_program_account(path: &syn::Path) -> ParseResult<ProgramAccountTy> {
    let account_ident = parse_account(path)?;
    Ok(ProgramAccountTy {
        account_type_path: account_ident,
    })
}

fn parse_program_account_zero_copy(path: &syn::Path) -> ParseResult<LoaderTy> {
    let account_ident = parse_account(path)?;
    Ok(LoaderTy {
        account_type_path: account_ident,
    })
}
fn parse_program_account_loader(path: &syn::Path) -> ParseResult<AccountLoaderTy> {
    let account_ident = parse_account(path)?;
    Ok(AccountLoaderTy {
        account_type_path: account_ident,
    })
}

fn parse_account_ty(path: &syn::Path) -> ParseResult<AccountTy> {
    let account_type_path = parse_account(path)?;
    let boxed = parser::tts_to_string(&path)
        .replace(' ', "")
        .starts_with("Box<Account<");
    Ok(AccountTy {
        account_type_path,
        boxed,
    })
}

fn parse_program_ty(path: &syn::Path) -> ParseResult<ProgramTy> {
    let account_type_path = parse_account(path)?;
    Ok(ProgramTy { account_type_path })
}

// TODO: this whole method is a hack. Do something more idiomatic.
fn parse_account(mut path: &syn::Path) -> ParseResult<syn::TypePath> {
    if parser::tts_to_string(path)
        .replace(' ', "")
        .starts_with("Box<Account<")
    {
        let segments = &path.segments[0];
        match &segments.arguments {
            syn::PathArguments::AngleBracketed(args) => {
                // Expected: <'info, MyType>.
                if args.args.len() != 1 {
                    return Err(ParseError::new(
                        args.args.span(),
                        "bracket arguments must be the lifetime and type",
                    ));
                }
                match &args.args[0] {
                    syn::GenericArgument::Type(syn::Type::Path(ty_path)) => {
                        path = &ty_path.path;
                    }
                    _ => {
                        return Err(ParseError::new(
                            args.args[1].span(),
                            "first bracket argument must be a lifetime",
                        ))
                    }
                }
            }
            _ => {
                return Err(ParseError::new(
                    segments.arguments.span(),
                    "expected angle brackets with a lifetime and type",
                ))
            }
        }
    }

    let segments = &path.segments[0];
    match &segments.arguments {
        syn::PathArguments::AngleBracketed(args) => {
            // Expected: <'info, MyType>.
            if args.args.len() != 2 {
                return Err(ParseError::new(
                    args.args.span(),
                    "bracket arguments must be the lifetime and type",
                ));
            }
            match &args.args[1] {
                syn::GenericArgument::Type(syn::Type::Path(ty_path)) => Ok(ty_path.clone()),
                _ => Err(ParseError::new(
                    args.args[1].span(),
                    "first bracket argument must be a lifetime",
                )),
            }
        }
        _ => Err(ParseError::new(
            segments.arguments.span(),
            "expected angle brackets with a lifetime and type",
        )),
    }
}

fn parse_sysvar(path: &syn::Path) -> ParseResult<SysvarTy> {
    let segments = &path.segments[0];
    let account_ident = match &segments.arguments {
        syn::PathArguments::AngleBracketed(args) => {
            // Expected: <'info, MyType>.
            if args.args.len() != 2 {
                return Err(ParseError::new(
                    args.args.span(),
                    "bracket arguments must be the lifetime and type",
                ));
            }
            match &args.args[1] {
                syn::GenericArgument::Type(syn::Type::Path(ty_path)) => {
                    // TODO: allow segmented paths.
                    if ty_path.path.segments.len() != 1 {
                        return Err(ParseError::new(
                            ty_path.path.span(),
                            "segmented paths are not currently allowed",
                        ));
                    }
                    let path_segment = &ty_path.path.segments[0];
                    path_segment.ident.clone()
                }
                _ => {
                    return Err(ParseError::new(
                        args.args[1].span(),
                        "first bracket argument must be a lifetime",
                    ))
                }
            }
        }
        _ => {
            return Err(ParseError::new(
                segments.arguments.span(),
                "expected angle brackets with a lifetime and type",
            ))
        }
    };
    let ty = match account_ident.to_string().as_str() {
        "Clock" => SysvarTy::Clock,
        "Rent" => SysvarTy::Rent,
        "EpochSchedule" => SysvarTy::EpochSchedule,
        "Fees" => SysvarTy::Fees,
        "RecentBlockhashes" => SysvarTy::RecentBlockhashes,
        "SlotHashes" => SysvarTy::SlotHashes,
        "SlotHistory" => SysvarTy::SlotHistory,
        "StakeHistory" => SysvarTy::StakeHistory,
        "Instructions" => SysvarTy::Instructions,
        "Rewards" => SysvarTy::Rewards,
        _ => {
            return Err(ParseError::new(
                account_ident.span(),
                "invalid sysvar provided",
            ))
        }
    };
    Ok(ty)
}
