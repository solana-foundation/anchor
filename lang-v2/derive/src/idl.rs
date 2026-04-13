//! IDL generation helpers.

use {quote::quote, syn::Type};

/// Convert a Rust type to its IDL JSON representation.
pub fn rust_type_to_idl(ty: &Type) -> String {
    type_str_to_idl(&quote!(#ty).to_string().replace(' ', ""))
}

/// Convert a stringified Rust type to IDL JSON.
fn type_str_to_idl(s: &str) -> String {
    match s {
        "u8" | "u16" | "u32" | "u64" | "u128" |
        "i8" | "i16" | "i32" | "i64" | "i128" |
        "bool" => format!("\"{s}\""),
        "String" | "string" => "\"string\"".into(),
        "Pubkey" | "Address" | "pubkey" => "\"pubkey\"".into(),
        "bytes" => "\"bytes\"".into(),
        _ if s.starts_with('[') && s.ends_with(']') && s.contains(';') => {
            let inner = &s[1..s.len()-1];
            if let Some((ty_part, n_part)) = inner.split_once(';') {
                let ty_json = type_str_to_idl(ty_part);
                // Try to parse as integer literal; if const expression, use 0 as placeholder
                let size = n_part.trim().parse::<usize>().unwrap_or(0);
                format!("{{\"array\":[{ty_json},{size}]}}")
            } else {
                format!("{{\"defined\":\"{s}\"}}")
            }
        }
        _ if s.starts_with("Vec<") => {
            let inner = s.strip_prefix("Vec<").unwrap().strip_suffix('>').unwrap();
            format!("{{\"vec\":{}}}", type_str_to_idl(inner))
        }
        _ if s.starts_with("Option<") => {
            let inner = s.strip_prefix("Option<").unwrap().strip_suffix('>').unwrap();
            format!("{{\"option\":{}}}", type_str_to_idl(inner))
        }
        _ if s.starts_with("Box<") => {
            let inner = s.strip_prefix("Box<").unwrap().strip_suffix('>').unwrap();
            type_str_to_idl(inner)
        }
        other => format!("{{\"defined\":\"{other}\"}}")
    }
}

/// Build IDL accounts JSON from parsed field metadata.
pub fn build_accounts_json(accounts: &[(String, bool, bool, Option<String>)]) -> String {
    let parts: Vec<String> = accounts.iter().map(|(name, writable, signer, address)| {
        let mut obj = format!("{{\"name\":\"{name}\"");
        if *writable { obj.push_str(",\"writable\":true"); }
        if *signer { obj.push_str(",\"signer\":true"); }
        if let Some(addr) = address {
            obj.push_str(&format!(",\"address\":\"{addr}\""));
        }
        obj.push('}');
        obj
    }).collect();
    format!("[{}]", parts.join(","))
}

/// Build IDL instruction args JSON from handler parameters.
pub fn build_args_json(args: &[(&syn::Ident, &Box<Type>)]) -> String {
    let parts: Vec<String> = args.iter().map(|(name, ty)| {
        let ty_json = rust_type_to_idl(ty);
        format!("{{\"name\":\"{name}\",\"type\":{ty_json}}}")
    }).collect();
    format!("[{}]", parts.join(","))
}

/// Build discriminator JSON from hash bytes.
pub fn disc_json(disc_bytes: &[u8]) -> String {
    let parts: Vec<String> = disc_bytes.iter().map(|b| b.to_string()).collect();
    format!("[{}]", parts.join(","))
}

/// Build IDL type definition JSON from struct fields.
pub fn build_type_json(name: &str, disc: &[u8], fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>) -> String {
    let disc_json = disc_json(disc);
    let field_jsons: Vec<String> = fields.iter().map(|f| {
        let fname = f.ident.as_ref().unwrap().to_string();
        let ftype = rust_type_to_idl(&f.ty);
        format!("{{\"name\":\"{fname}\",\"type\":{ftype}}}")
    }).collect();
    format!(
        "{{\"name\":\"{name}\",\"discriminator\":{disc_json},\"type\":{{\"kind\":\"struct\",\"fields\":[{}]}}}}",
        field_jsons.join(",")
    )
}
