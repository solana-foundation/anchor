pub mod constraints;
#[cfg(feature = "event-cpi")]
pub mod event_cpi;

use crate::parser::docs;
use crate::*;
use syn::parse::{Error as ParseError, Result as ParseResult};
use syn::Path;

pub fn parse(accounts_struct: &syn::ItemStruct) -> ParseResult<AccountsStruct> {
    let instruction_api: Option<Punctuated<Expr, Comma>> = accounts_struct
        .attrs
        .iter()
        .find(|a| {
            a.path
                .get_ident()
                .is_some_and(|ident| ident == "instruction")
        })
        .map(|ix_attr| ix_attr.parse_args_with(Punctuated::<Expr, Comma>::parse_terminated))
        .transpose()?;

    #[cfg(feature = "event-cpi")]
    let accounts_struct = {
        let is_event_cpi = accounts_struct
            .attrs
            .iter()
            .filter_map(|attr| attr.path.get_ident())
            .any(|ident| *ident == "event_cpi");
        if is_event_cpi {
            event_cpi::add_event_cpi_accounts(accounts_struct)?
        } else {
            accounts_struct.clone()
        }
    };
    #[cfg(not(feature = "event-cpi"))]
    let accounts_struct = accounts_struct.clone();

    let fields = match &accounts_struct.fields {
        syn::Fields::Named(fields) => fields
            .named
            .iter()
            .map(parse_account_field)
            .collect::<ParseResult<Vec<AccountField>>>()?,
        _ => {
            return Err(ParseError::new_spanned(
                &accounts_struct.fields,
                "fields must be named",
            ))
        }
    };

    constraints_cross_checks(&fields)?;

    Ok(AccountsStruct::new(
        accounts_struct,
        fields,
        instruction_api,
    ))
}

fn constraints_cross_checks(fields: &[AccountField]) -> ParseResult<()> {
    // COMMON ERROR MESSAGE
    let message = |constraint: &str, field: &str, required: bool| {
        if required {
            format! {
                "a non-optional {constraint} constraint requires \
                a non-optional {field} field to exist in the account \
                validation struct. Use the Program type to add \
                the {field} field to your validation struct."
            }
        } else {
            format! {
                "an optional {constraint} constraint requires \
                an optional or required {field} field to exist \
                in the account validation struct. Use the Program type \
                to add the {field} field to your validation struct."
            }
        }
    };

    // INIT
    let mut required_init = false;
    let init_fields: Vec<&Field> = fields
        .iter()
        .filter_map(|f| match f {
            AccountField::Field(field) if field.constraints.init.is_some() => {
                if !field.is_optional {
                    required_init = true
                }
                Some(field)
            }
            _ => None,
        })
        .collect();

    if !init_fields.is_empty() {
        // init needs system program.

        if !fields
            .iter()
            // ensures that a non optional `system_program` is present with non optional `init`
            .any(|f| f.ident() == "system_program" && !(required_init && f.is_optional()))
        {
            return Err(ParseError::new(
                init_fields[0].ident.span(),
                message("init", "system_program", required_init),
            ));
        }

        let kind = &init_fields[0].constraints.init.as_ref().unwrap().kind;
        // init token/a_token/mint needs token program.
        match kind {
            InitKind::Program { .. } | InitKind::Interface { .. } => (),
            InitKind::Token { token_program, .. }
            | InitKind::AssociatedToken { token_program, .. }
            | InitKind::Mint { token_program, .. } => {
                // is the token_program constraint specified?
                let token_program_field = if let Some(token_program_id) = token_program {
                    // if so, is it present in the struct?
                    token_program_id.to_token_stream().to_string()
                } else {
                    // if not, look for the token_program field
                    "token_program".to_string()
                };
                if !fields.iter().any(|f| {
                    f.ident() == &token_program_field && !(required_init && f.is_optional())
                }) {
                    return Err(ParseError::new(
                        init_fields[0].ident.span(),
                        message("init", &token_program_field, required_init),
                    ));
                }
            }
        }

        // a_token needs associated token program.
        if let InitKind::AssociatedToken { .. } = kind {
            if !fields.iter().any(|f| {
                f.ident() == "associated_token_program" && !(required_init && f.is_optional())
            }) {
                return Err(ParseError::new(
                    init_fields[0].ident.span(),
                    message("init", "associated_token_program", required_init),
                ));
            }
        }

        for (pos, field) in init_fields.iter().enumerate() {
            // Get payer for init-ed account
            let associated_payer_name = match field.constraints.init.clone().unwrap().payer {
                // composite payer, check not supported
                Expr::Field(_) => continue,
                // method call, check not supported
                Expr::MethodCall(_) => continue,
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
                    } else if associated_payer_field.is_optional && required_init {
                        return Err(ParseError::new(
                            field.ident.span(),
                            "the payer specified for a required init constraint must be required.",
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
            match &field.constraints.init.as_ref().unwrap().kind {
                // This doesn't catch cases like account.key() or account.key.
                // My guess is that doesn't happen often and we can revisit
                // this if I'm wrong.
                InitKind::Token { mint, .. } | InitKind::AssociatedToken { mint, .. } => {
                    if !fields.iter().any(|f| {
                        f.ident()
                            .to_string()
                            .starts_with(&mint.to_token_stream().to_string())
                    }) {
                        return Err(ParseError::new(
                            field.ident.span(),
                            "the mint constraint has to be an account field for token initializations (not a public key)",
                        ));
                    }
                }

                // Make sure initialiazed token accounts are always declared after their corresponding mint.
                InitKind::Mint { .. } => {
                    if init_fields.iter().enumerate().any(|(f_pos, f)| {
                        match &f.constraints.init.as_ref().unwrap().kind {
                            InitKind::Token { mint, .. }
                            | InitKind::AssociatedToken { mint, .. } => {
                                field.ident == mint.to_token_stream().to_string() && pos > f_pos
                            }
                            _ => false,
                        }
                    }) {
                        return Err(ParseError::new(
                            field.ident.span(),
                            "because of the init constraint, the mint has to be declared before the corresponding token account",
                        ));
                    }
                }
                _ => (),
            }
        }
    }

    // REALLOC
    let mut required_realloc = false;
    let realloc_fields: Vec<&Field> = fields
        .iter()
        .filter_map(|f| match f {
            AccountField::Field(field) if field.constraints.realloc.is_some() => {
                if !field.is_optional {
                    required_realloc = true
                }
                Some(field)
            }
            _ => None,
        })
        .collect();

    if !realloc_fields.is_empty() {
        // realloc needs system program.
        if !fields
            .iter()
            .any(|f| f.ident() == "system_program" && !(required_realloc && f.is_optional()))
        {
            return Err(ParseError::new(
                realloc_fields[0].ident.span(),
                message("realloc", "system_program", required_realloc),
            ));
        }

        for field in realloc_fields {
            // Get allocator for realloc-ed account
            let associated_payer_name = match field.constraints.realloc.clone().unwrap().payer {
                // composite allocator, check not supported
                Expr::Field(_) => continue,
                // method call, check not supported
                Expr::MethodCall(_) => continue,
                field_name => field_name.to_token_stream().to_string(),
            };

            // Check allocator is mutable
            let associated_payer_field = fields.iter().find_map(|f| match f {
                AccountField::Field(field) if *f.ident() == associated_payer_name => Some(field),
                _ => None,
            });

            match associated_payer_field {
                Some(associated_payer_field) => {
                    if !associated_payer_field.constraints.is_mutable() {
                        return Err(ParseError::new(
                            field.ident.span(),
                            "the realloc::payer specified for an realloc constraint must be mutable.",
                        ));
                    } else if associated_payer_field.is_optional && required_realloc {
                        return Err(ParseError::new(
                            field.ident.span(),
                            "the realloc::payer specified for a required realloc constraint must be required.",
                        ));
                    }
                }
                _ => {
                    return Err(ParseError::new(
                        field.ident.span(),
                        "the realloc::payer specified does not exist.",
                    ));
                }
            }
        }
    }

    Ok(())
}

pub fn parse_account_field(f: &syn::Field) -> ParseResult<AccountField> {
    let ident = f.ident.clone().unwrap();
    let docs = docs::parse(&f.attrs);
    let account_field = match is_field_primitive(f)? {
        true => {
            let (ty, is_optional) = parse_ty(f)?;
            let account_constraints = constraints::parse(f, Some(&ty))?;
            AccountField::Field(Field {
                ident,
                ty,
                is_optional,
                constraints: account_constraints,
                docs,
            })
        }
        false => {
            let (_, optional, _) = ident_string(f)?;
            if optional {
                return Err(ParseError::new(
                    f.ty.span(),
                    "Cannot have Optional composite accounts",
                ));
            }
            let account_constraints = constraints::parse(f, None)?;
            AccountField::CompositeField(CompositeField {
                ident,
                constraints: account_constraints,
                symbol: ident_string(f)?.0,
                raw_field: f.clone(),
                docs,
            })
        }
    };
    Ok(account_field)
}

fn is_field_primitive(f: &syn::Field) -> ParseResult<bool> {
    let r = matches!(
        ident_string(f)?.0.as_str(),
        "Sysvar"
            | "AccountInfo"
            | "UncheckedAccount"
            | "AccountLoader"
            | "Account"
            | "LazyAccount"
            | "Program"
            | "Interface"
            | "InterfaceAccount"
            | "Signer"
            | "SystemAccount"
            | "ProgramData"
    );
    Ok(r)
}

fn parse_ty(f: &syn::Field) -> ParseResult<(Ty, bool)> {
    let (ident, optional, path) = ident_string(f)?;
    let ty = match ident.as_str() {
        "Sysvar" => Ty::Sysvar(parse_sysvar(&path)?),
        "AccountInfo" => Ty::AccountInfo,
        "UncheckedAccount" => Ty::UncheckedAccount,
        "AccountLoader" => Ty::AccountLoader(parse_program_account_loader(&path)?),
        "Account" => Ty::Account(parse_account_ty(&path)?),
        "LazyAccount" => Ty::LazyAccount(parse_lazy_account_ty(&path)?),
        "Program" => Ty::Program(parse_program_ty(&path)?),
        "Interface" => Ty::Interface(parse_interface_ty(&path)?),
        "InterfaceAccount" => Ty::InterfaceAccount(parse_interface_account_ty(&path)?),
        "Signer" => Ty::Signer,
        "SystemAccount" => Ty::SystemAccount,
        "ProgramData" => Ty::ProgramData,
        _ => return Err(ParseError::new(f.ty.span(), "invalid account type given")),
    };

    Ok((ty, optional))
}

fn option_to_inner_path(path: &Path) -> ParseResult<Path> {
    let segment_0 = path.segments[0].clone();
    match segment_0.arguments {
        syn::PathArguments::AngleBracketed(args) => {
            if args.args.len() != 1 {
                return Err(ParseError::new(
                    args.args.span(),
                    "can only have one argument in option",
                ));
            }
            match &args.args[0] {
                syn::GenericArgument::Type(syn::Type::Path(ty_path)) => Ok(ty_path.path.clone()),
                _ => Err(ParseError::new(
                    args.args[1].span(),
                    "first bracket argument must be a lifetime",
                )),
            }
        }
        _ => Err(ParseError::new(
            segment_0.arguments.span(),
            "expected angle brackets with a lifetime and type",
        )),
    }
}

fn ident_string(f: &syn::Field) -> ParseResult<(String, bool, Path)> {
    let mut path = match &f.ty {
        syn::Type::Path(ty_path) => ty_path.path.clone(),
        _ => return Err(ParseError::new(f.ty.span(), "invalid account type given")),
    };
    let mut optional = false;
    if parser::tts_to_string(&path)
        .replace(' ', "")
        .starts_with("Option<")
    {
        path = option_to_inner_path(&path)?;
        optional = true;
    }
    if parser::tts_to_string(&path)
        .replace(' ', "")
        .starts_with("Box<Account<")
    {
        return Ok(("Account".to_string(), optional, path));
    }
    if parser::tts_to_string(&path)
        .replace(' ', "")
        .starts_with("Box<InterfaceAccount<")
    {
        return Ok(("InterfaceAccount".to_string(), optional, path));
    }
    // TODO: allow segmented paths.
    if path.segments.len() != 1 {
        return Err(ParseError::new(
            f.ty.span(),
            "segmented paths are not currently allowed",
        ));
    }

    let segments = &path.segments[0];
    Ok((segments.ident.to_string(), optional, path))
}

fn parse_program_account_loader(path: &syn::Path) -> ParseResult<AccountLoaderTy> {
    let account_ident = parse_account(path)?;
    Ok(AccountLoaderTy {
        account_type_path: account_ident,
    })
}

fn parse_account_ty(path: &syn::Path) -> ParseResult<AccountTy> {
    let account_type_path = parse_account(path)?;
    let boxed = parser::tts_to_string(path)
        .replace(' ', "")
        .starts_with("Box<Account<");
    Ok(AccountTy {
        account_type_path,
        boxed,
    })
}

fn parse_lazy_account_ty(path: &syn::Path) -> ParseResult<LazyAccountTy> {
    let account_type_path = parse_account(path)?;
    Ok(LazyAccountTy { account_type_path })
}

fn parse_interface_account_ty(path: &syn::Path) -> ParseResult<InterfaceAccountTy> {
    let account_type_path = parse_account(path)?;
    let boxed = parser::tts_to_string(path)
        .replace(' ', "")
        .starts_with("Box<InterfaceAccount<");
    Ok(InterfaceAccountTy {
        account_type_path,
        boxed,
    })
}

fn parse_program_ty(path: &syn::Path) -> ParseResult<ProgramTy> {
    let account_type_path = parse_account(path)?;
    Ok(ProgramTy { account_type_path })
}

fn parse_interface_ty(path: &syn::Path) -> ParseResult<InterfaceTy> {
    let account_type_path = parse_account(path)?;
    Ok(InterfaceTy { account_type_path })
}

// TODO: this whole method is a hack. Do something more idiomatic.
fn parse_account(mut path: &syn::Path) -> ParseResult<syn::TypePath> {
    let path_str = parser::tts_to_string(path).replace(' ', "");
    if path_str.starts_with("Box<Account<") || path_str.starts_with("Box<InterfaceAccount<") {
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
