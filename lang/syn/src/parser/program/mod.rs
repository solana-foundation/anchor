use crate::parser::docs;
use crate::Program;
use syn::parse::{Error as ParseError, Result as ParseResult};
use syn::spanned::Spanned;

mod instructions;

pub fn parse(program_mod: syn::ItemMod) -> ParseResult<Program> {
    let docs = docs::parse(&program_mod.attrs);
    let (ixs, fallback_fn) = instructions::parse(&program_mod)?;
    let program_id = parse_program_id(&program_mod);
    Ok(Program {
        ixs,
        name: program_mod.ident.clone(),
        docs,
        program_mod,
        fallback_fn,
        program_id,
    })
}

/// Scans the `#[program]` module items for an inline `const ID: Pubkey = ...`
/// declaration. Returns `Some(expr)` if found, or `None` if the program relies
/// on `declare_id!` at the crate level.
fn parse_program_id(program_mod: &syn::ItemMod) -> Option<proc_macro2::TokenStream> {
    let items = &program_mod.content.as_ref()?.1;
    for item in items {
        if let syn::Item::Const(c) = item {
            if c.ident == "ID"
                && matches!(*c.ty, syn::Type::Path(ref p) if p.path.segments.last().is_some_and(|s| s.ident == "Pubkey"))
            {
                let expr = &c.expr;
                return Some(quote::quote! { #expr });
            }
        }
    }
    None
}

fn ctx_accounts_ident(path_ty: &syn::PatType) -> ParseResult<proc_macro2::Ident> {
    let p = match &*path_ty.ty {
        syn::Type::Path(p) => &p.path,
        _ => return Err(ParseError::new(path_ty.ty.span(), "invalid type")),
    };
    let segment = p
        .segments
        .first()
        .ok_or_else(|| ParseError::new(p.segments.span(), "expected generic arguments here"))?;

    let generic_args = match &segment.arguments {
        syn::PathArguments::AngleBracketed(args) => args,
        _ => return Err(ParseError::new(path_ty.span(), "missing accounts context")),
    };
    let generic_ty = generic_args
        .args
        .iter()
        .filter_map(|arg| match arg {
            syn::GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
        .next()
        .ok_or_else(|| ParseError::new(generic_args.span(), "expected Accounts type"))?;

    let path = match generic_ty {
        syn::Type::Path(ty_path) => &ty_path.path,
        _ => {
            return Err(ParseError::new(
                generic_ty.span(),
                "expected Accounts struct type",
            ))
        }
    };
    Ok(path.segments[0].ident.clone())
}
