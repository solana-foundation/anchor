use syn::{ext::IdentExt, parse::{Parse, ParseStream}, Attribute, Expr, Ident, Token, Type};

pub struct AccountAttrs {
    pub is_mut: bool,
    pub is_init: bool,
    pub has_bump: bool,
    pub payer: Option<Ident>,
    pub space: Option<Expr>,
    pub seeds: Option<Vec<Expr>>,
    pub has_one: Vec<Ident>,
    pub address: Option<Expr>,
    pub close: Option<Ident>,
}

pub fn parse_account_attrs(attrs: &[Attribute]) -> AccountAttrs {
    let mut result = AccountAttrs {
        is_mut: false,
        is_init: false,
        has_bump: false,
        payer: None,
        space: None,
        seeds: None,
        has_one: Vec::new(),
        address: None,
        close: None,
    };

    for attr in attrs {
        if !attr.path().is_ident("account") {
            continue;
        }
        let _ = attr.parse_args_with(|input: ParseStream| {
            while !input.is_empty() {
                let ident = Ident::parse_any(input)?;
                match ident.to_string().as_str() {
                    "mut" => result.is_mut = true,
                    "init" => {
                        result.is_init = true;
                        result.is_mut = true;
                    }
                    "bump" => result.has_bump = true,
                    "signer" => {}
                    "payer" => {
                        input.parse::<Token![=]>()?;
                        result.payer = Some(input.parse()?);
                    }
                    "space" => {
                        input.parse::<Token![=]>()?;
                        result.space = Some(input.parse()?);
                    }
                    "seeds" => {
                        input.parse::<Token![=]>()?;
                        let content;
                        syn::bracketed!(content in input);
                        let seeds = content
                            .parse_terminated(Expr::parse, Token![,])?
                            .into_iter()
                            .collect();
                        result.seeds = Some(seeds);
                    }
                    "has_one" => {
                        input.parse::<Token![=]>()?;
                        result.has_one.push(input.parse()?);
                    }
                    "address" => {
                        input.parse::<Token![=]>()?;
                        result.address = Some(input.parse()?);
                    }
                    "close" => {
                        input.parse::<Token![=]>()?;
                        result.close = Some(input.parse()?);
                    }
                    _ => {}
                }
                if !input.is_empty() {
                    input.parse::<Token![,]>()?;
                }
            }
            Ok(())
        });
    }
    result
}

pub fn field_ty_str(ty: &Type) -> String {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident.to_string();
        }
    }
    String::new()
}

pub fn is_nested_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Nested";
        }
    }
    false
}
