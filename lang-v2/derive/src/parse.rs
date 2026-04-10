use syn::{ext::IdentExt, parse::{Parse, ParseStream}, Attribute, Expr, Ident, Token, Type};

pub struct AccountAttrs {
    pub is_mut: bool,
    pub is_init: bool,
    pub is_init_if_needed: bool,
    pub has_bump: bool,
    pub payer: Option<Ident>,
    pub space: Option<Expr>,
    pub seeds: Option<Vec<Expr>>,
    pub has_one: Vec<Ident>,
    pub address: Option<Expr>,
    pub close: Option<Ident>,
    pub constraint: Option<Expr>,
    pub constraint_error: Option<Expr>,
    pub realloc: Option<Expr>,
    pub realloc_payer: Option<Ident>,
    pub realloc_zero: bool,
}

pub fn parse_account_attrs(attrs: &[Attribute]) -> AccountAttrs {
    let mut result = AccountAttrs {
        is_mut: false,
        is_init: false,
        is_init_if_needed: false,
        has_bump: false,
        payer: None,
        space: None,
        seeds: None,
        has_one: Vec::new(),
        address: None,
        close: None,
        constraint: None,
        constraint_error: None,
        realloc: None,
        realloc_payer: None,
        realloc_zero: false,
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
                    "init_if_needed" => {
                        result.is_init_if_needed = true;
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
                    "realloc" => {
                        input.parse::<Token![=]>()?;
                        result.realloc = Some(input.parse()?);
                        result.is_mut = true;
                    }
                    "realloc_payer" => {
                        input.parse::<Token![=]>()?;
                        result.realloc_payer = Some(input.parse()?);
                    }
                    "realloc_zero" => {
                        input.parse::<Token![=]>()?;
                        let val: syn::LitBool = input.parse()?;
                        result.realloc_zero = val.value;
                    }
                    "close" => {
                        input.parse::<Token![=]>()?;
                        result.close = Some(input.parse()?);
                    }
                    "constraint" => {
                        input.parse::<Token![=]>()?;
                        result.constraint = Some(input.parse()?);
                        // Optional: @ ErrorExpr
                        if input.peek(Token![@]) {
                            input.parse::<Token![@]>()?;
                            result.constraint_error = Some(input.parse()?);
                        }
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

/// Extract the inner `T` from `BorshAccount<T>` or `Account<T>`.
pub fn extract_inner_data_type(ty: &Type) -> Option<proc_macro2::TokenStream> {
    use quote::quote;
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            let name = seg.ident.to_string();
            if name == "BorshAccount" || name == "Account" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(quote! { #inner });
                    }
                }
            }
        }
    }
    None
}

pub fn is_nested_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Nested";
        }
    }
    false
}

/// Extract the well-known address from `Program<T>` types.
/// Returns the base58 address string for known program types (System, Token, etc.).
pub fn extract_program_address(ty: &Type) -> Option<String> {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Program" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(Type::Path(inner_tp))) = args.args.first() {
                        if let Some(inner_seg) = inner_tp.path.segments.last() {
                            return match inner_seg.ident.to_string().as_str() {
                                "System" => Some("11111111111111111111111111111111".to_string()),
                                "Token" => Some("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string()),
                                "Token2022" => Some("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb".to_string()),
                                "AssociatedToken" => Some("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL".to_string()),
                                "Memo" => Some("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr".to_string()),
                                _ => None,
                            };
                        }
                    }
                }
            }
        }
    }
    None
}
