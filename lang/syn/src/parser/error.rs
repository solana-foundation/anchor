use crate::{Error, ErrorArgs, ErrorCode};
use syn::parse::{Parse, Result as ParseResult};
use syn::Expr;

// Removes any internal #[msg] attributes, as they are inert.
pub fn parse(error_enum: &mut syn::ItemEnum, args: Option<ErrorArgs>) -> ParseResult<Error> {
    let ident = error_enum.ident.clone();
    let mut last_discriminant = 0;
    let mut codes: Vec<ErrorCode> = Vec::new();
    for variant in error_enum.variants.iter_mut() {
        let msg = parse_error_attribute(variant)?;
        let ident = variant.ident.clone();
        let id = match &variant.discriminant {
            None => last_discriminant,
            Some((_, disc)) => match disc {
                syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
                    syn::Lit::Int(int) => int.base10_parse::<u32>()?,
                    _ => {
                        return Err(syn::Error::new_spanned(
                            disc,
                            "Invalid error discriminant: must be an integer",
                        ))
                    }
                },
                _ => {
                    return Err(syn::Error::new_spanned(
                        disc,
                        "Invalid error discriminant: must be a literal",
                    ))
                }
            },
        };
        last_discriminant = id + 1;

        // Remove any non-doc attributes on the error variant.
        variant
            .attrs
            .retain(|attr| attr.path.segments[0].ident == "doc");

        codes.push(ErrorCode { id, ident, msg });
    }
    Ok(Error {
        name: error_enum.ident.to_string(),
        raw_enum: error_enum.clone(),
        ident,
        codes,
        args,
    })
}

fn parse_error_attribute(variant: &syn::Variant) -> ParseResult<Option<String>> {
    let attrs = variant
        .attrs
        .iter()
        .filter(|attr| attr.path.segments[0].ident != "doc")
        .collect::<Vec<_>>();
    match attrs.len() {
        0 => Ok(None),
        1 => {
            let attr = &attrs[0];
            let attr_str = attr.path.segments[0].ident.to_string();
            if attr_str != "msg" {
                return Err(syn::Error::new_spanned(
                    attr,
                    "Use msg to specify error strings",
                ));
            }

            let mut tts = attr.tokens.clone().into_iter();
            let g_stream = match tts.next() {
                Some(proc_macro2::TokenTree::Group(g)) => g.stream(),
                Some(t) => return Err(syn::Error::new_spanned(t, "Invalid syntax")),
                None => return Err(syn::Error::new_spanned(attr, "Must have a token group")),
            };

            let msg = match g_stream.into_iter().next() {
                None => {
                    return Err(syn::Error::new_spanned(
                        attr,
                        "Must specify a message string",
                    ))
                }
                Some(msg) => msg.to_string().replace('\"', ""),
            };

            Ok(Some(msg))
        }
        _ => Err(syn::Error::new_spanned(
            variant,
            "Too many attributes found. Use `msg` to specify error strings",
        )),
    }
}

pub struct ErrorInput {
    pub error_code: Expr,
}

impl Parse for ErrorInput {
    fn parse(stream: syn::parse::ParseStream) -> ParseResult<Self> {
        let error_code = stream.call(Expr::parse)?;
        Ok(Self { error_code })
    }
}
