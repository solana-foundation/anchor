use syn::{Expr, ExprLit, Lit::Str};

/// Gather all docstrings/`#[doc = "STRING"]` attributes, ignoring `CHECK:` lines.
/// Will return `None` if there are no docstrings.
pub fn parse(attrs: &[syn::Attribute]) -> Option<Vec<String>> {
    let mut doc_strings = Vec::new();
    for_each_docstring(attrs, |string| {
        if !string.starts_with("CHECK:") {
            doc_strings.push(string.to_string());
        }
    });

    if doc_strings.is_empty() {
        None
    } else {
        Some(doc_strings)
    }
}

/// Check if any of these attributes are a docstring containing a `CHECK:`
pub fn has_check(attrs: &[syn::Attribute]) -> bool {
    let mut has_check = false;
    for_each_docstring(attrs, |string| {
        if string.contains("CHECK") {
            has_check = true;
        }
    });
    has_check
}

fn for_each_docstring<F>(attrs: &[syn::Attribute], mut f: F)
where
    F: FnMut(String),
{
    for attr in attrs {
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("doc") {
                if let Ok(Expr::Lit(ExprLit { lit: Str(doc), .. })) =
                    meta.value().and_then(|v| v.parse())
                {
                    f(doc.value())
                }
            }
            Ok(())
        });
    }
}
