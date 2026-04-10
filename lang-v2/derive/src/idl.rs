//! IDL generation helpers.
//!
//! Generates IDL JSON fragments from macro metadata. Gated behind
//! `#[cfg(feature = "idl-build")]` in the generated code.

use {quote::quote, syn::Type};

/// Convert a Rust type to its IDL JSON representation.
pub fn rust_type_to_idl(ty: &Type) -> String {
    let s = quote!(#ty).to_string().replace(' ', "");
    match s.as_str() {
        "u8" => "\"u8\"".into(),
        "u16" => "\"u16\"".into(),
        "u32" => "\"u32\"".into(),
        "u64" => "\"u64\"".into(),
        "u128" => "\"u128\"".into(),
        "i8" => "\"i8\"".into(),
        "i16" => "\"i16\"".into(),
        "i32" => "\"i32\"".into(),
        "i64" => "\"i64\"".into(),
        "i128" => "\"i128\"".into(),
        "bool" => "\"bool\"".into(),
        "String" => "\"string\"".into(),
        "Pubkey" | "Address" => "\"pubkey\"".into(),
        _ if s.starts_with("[u8;") => {
            let n = s.trim_start_matches("[u8;").trim_end_matches(']');
            format!("{{\"array\":[\"u8\",{n}]}}")
        }
        _ => format!("{{\"defined\":\"{s}\"}}")
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
