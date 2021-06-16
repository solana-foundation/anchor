use crate::{
    AccountField, AccountsStruct, CompositeField, CpiAccountTy, CpiStateTy, Field, LoaderTy,
    ProgramAccountTy, ProgramStateTy, SysvarTy, Ty,
};
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
        .filter(|a| {
            a.path
                .get_ident()
                .map_or(false, |ident| ident == "instruction")
        })
        .next()
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
    Ok(AccountsStruct::new(strct.clone(), fields, instruction_api))
}

pub fn parse_account_field(f: &syn::Field, has_instruction_api: bool) -> ParseResult<AccountField> {
    let ident = f.ident.clone().unwrap();
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
            })
        }
    };
    Ok(account_field)
}

fn is_field_primitive(f: &syn::Field) -> ParseResult<bool> {
    let r = match ident_string(f)?.as_str() {
        "ProgramState" | "ProgramAccount" | "CpiAccount" | "Sysvar" | "AccountInfo"
        | "CpiState" | "Loader" => true,
        _ => false,
    };
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
        "Loader" => Ty::Loader(parse_program_account_zero_copy(&path)?),
        _ => return Err(ParseError::new(f.ty.span(), "invalid account type given")),
    };

    Ok(ty)
}

fn ident_string(f: &syn::Field) -> ParseResult<String> {
    let path = match &f.ty {
        syn::Type::Path(ty_path) => ty_path.path.clone(),
        _ => return Err(ParseError::new(f.ty.span(), "invalid type")),
    };
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
    let account_ident = parse_account(&path)?;
    Ok(ProgramStateTy { account_ident })
}

fn parse_cpi_state(path: &syn::Path) -> ParseResult<CpiStateTy> {
    let account_ident = parse_account(&path)?;
    Ok(CpiStateTy { account_ident })
}

fn parse_cpi_account(path: &syn::Path) -> ParseResult<CpiAccountTy> {
    let account_ident = parse_account(path)?;
    Ok(CpiAccountTy { account_ident })
}

fn parse_program_account(path: &syn::Path) -> ParseResult<ProgramAccountTy> {
    let account_ident = parse_account(path)?;
    Ok(ProgramAccountTy { account_ident })
}

fn parse_program_account_zero_copy(path: &syn::Path) -> ParseResult<LoaderTy> {
    let account_ident = parse_account(path)?;
    Ok(LoaderTy { account_ident })
}

fn parse_account(path: &syn::Path) -> ParseResult<syn::Ident> {
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
                syn::GenericArgument::Type(syn::Type::Path(ty_path)) => {
                    // TODO: allow segmented paths.
                    if ty_path.path.segments.len() != 1 {
                        return Err(ParseError::new(
                            ty_path.path.span(),
                            "segmented paths are not currently allowed",
                        ));
                    }

                    let path_segment = &ty_path.path.segments[0];
                    Ok(path_segment.ident.clone())
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
