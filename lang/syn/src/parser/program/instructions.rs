use crate::parser::docs;
use crate::parser::program::ctx_accounts_ident;
use crate::parser::spl_interface;
use crate::{FallbackFn, Ix, IxArg, IxReturn, Overrides};
use syn::parse::{Error as ParseError, Result as ParseResult};
use syn::spanned::Spanned;
use syn::Attribute;

// Parse all non-state ix handlers from the program mod definition.
pub fn parse(program_mod: &syn::ItemMod) -> ParseResult<(Vec<Ix>, Option<FallbackFn>)> {
    let mod_content = &program_mod
        .content
        .as_ref()
        .ok_or_else(|| ParseError::new(program_mod.span(), "program content not provided"))?
        .1;

    let ixs = mod_content
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(item_fn) => {
                let (ctx, _) = parse_args(item_fn).ok()?;
                ctx_accounts_ident(&ctx.raw_arg).ok()?;
                Some(item_fn)
            }
            _ => None,
        })
        .map(|method: &syn::ItemFn| {
            let (ctx, args) = parse_args(method)?;
            let overrides = parse_overrides(&method.attrs)?;
            let interface_discriminator = spl_interface::parse(&method.attrs);
            let docs = docs::parse(&method.attrs);
            let cfgs = parse_cfg(method);
            let returns = parse_return(method)?;
            let anchor_ident = ctx_accounts_ident(&ctx.raw_arg)?;
            Ok(Ix {
                raw_method: method.clone(),
                ident: method.sig.ident.clone(),
                docs,
                cfgs,
                args,
                anchor_ident,
                returns,
                interface_discriminator,
                overrides,
            })
        })
        .collect::<ParseResult<Vec<Ix>>>()?;

    let fallback_fn = {
        let fallback_fns = mod_content
            .iter()
            .filter_map(|item| match item {
                syn::Item::Fn(item_fn) => {
                    let (ctx, _args) = parse_args(item_fn).ok()?;
                    if ctx_accounts_ident(&ctx.raw_arg).is_ok() {
                        return None;
                    }
                    Some(item_fn)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if fallback_fns.len() > 1 {
            return Err(ParseError::new(
                fallback_fns[0].span(),
                "More than one fallback function found",
            ));
        }
        fallback_fns
            .first()
            .map(|method: &&syn::ItemFn| FallbackFn {
                raw_method: (*method).clone(),
            })
    };

    Ok((ixs, fallback_fn))
}

/// Parse overrides from the `#[instruction]` attribute proc-macro.
fn parse_overrides(attrs: &[syn::Attribute]) -> ParseResult<Option<Overrides>> {
    attrs
        .iter()
        .find(|attr| match attr.path.segments.last() {
            Some(seg) => seg.ident == "instruction",
            _ => false,
        })
        .map(|attr| attr.parse_args())
        .transpose()
}

pub fn parse_args(method: &syn::ItemFn) -> ParseResult<(IxArg, Vec<IxArg>)> {
    let mut args: Vec<IxArg> = method
        .sig
        .inputs
        .iter()
        .map(|arg: &syn::FnArg| match arg {
            syn::FnArg::Typed(arg) => {
                let docs = docs::parse(&arg.attrs);
                let ident = match &*arg.pat {
                    syn::Pat::Ident(ident) => &ident.ident,
                    _ => return Err(ParseError::new(arg.pat.span(), "expected argument name")),
                };
                Ok(IxArg {
                    name: ident.clone(),
                    docs,
                    raw_arg: arg.clone(),
                })
            }
            syn::FnArg::Receiver(_) => Err(ParseError::new(
                arg.span(),
                "expected a typed argument not self",
            )),
        })
        .collect::<ParseResult<_>>()?;

    // Remove the Context argument
    let ctx = args.remove(0);

    Ok((ctx, args))
}

pub fn parse_return(method: &syn::ItemFn) -> ParseResult<IxReturn> {
    match method.sig.output {
        syn::ReturnType::Type(_, ref ty) => {
            let ty = match ty.as_ref() {
                syn::Type::Path(ty) => ty,
                _ => return Err(ParseError::new(ty.span(), "expected a return type")),
            };
            // Assume unit return by default
            let default_generic_arg = syn::GenericArgument::Type(syn::parse_str("()").unwrap());
            let generic_args = match &ty.path.segments.last().unwrap().arguments {
                syn::PathArguments::AngleBracketed(params) => {
                    params.args.iter().next_back().unwrap()
                }
                _ => &default_generic_arg,
            };
            let ty = match generic_args {
                syn::GenericArgument::Type(ty) => ty.clone(),
                _ => {
                    return Err(ParseError::new(
                        ty.span(),
                        "expected generic return type to be a type",
                    ))
                }
            };
            Ok(IxReturn { ty })
        }
        _ => Err(ParseError::new(
            method.sig.output.span(),
            "expected a return type",
        )),
    }
}

fn parse_cfg(method: &syn::ItemFn) -> Vec<Attribute> {
    method
        .attrs
        .iter()
        .filter_map(|attr| match attr.path.is_ident("cfg") {
            true => Some(attr.to_owned()),
            false => None,
        })
        .collect()
}
