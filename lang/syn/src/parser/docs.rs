use syn::{Lit::Str, Meta::NameValue};

// returns vec of doc strings
pub fn parse(attrs: &[syn::Attribute]) -> Option<Vec<String>> {
    let doc_strings: Vec<String> = attrs
        .iter()
        .filter_map(|attr| match attr.parse_meta() {
            Ok(NameValue(meta)) => {
                if meta.path.is_ident("doc") {
                    if let Str(doc) = meta.lit {
                        let val = doc.value().trim().to_string();
                        if val.starts_with("CHECK:") {
                            return None;
                        }
                        return Some(val);
                    }
                }
                None
            }
            _ => None,
        })
        .collect();
    if doc_strings.is_empty() {
        None
    } else {
        Some(doc_strings)
    }
}
