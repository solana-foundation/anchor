//! Generates `.equ` assembly constants from `#[account]` structs in lib.rs.
//!
//! For each struct annotated with `#[account]`:
//! - `StructName__SIZE` — `size_of::<Struct>()`
//! - `StructName__DISC_SIZE` — 8 (anchor discriminator)
//! - `StructName__INIT_SPACE` — 8 + size_of (total account allocation)
//! - `StructName__field` — byte offset of each field
//!
//! Offsets are computed from `#[repr(C)]` layout rules (which `#[account]`
//! enforces via bytemuck Pod). Fields must be primitive numeric types or
//! fixed-size arrays of them — no generics, no references.

use std::path::Path;

/// Parse `lib.rs` and generate `.equ` preamble for all `#[account]` structs.
pub fn generate(lib_rs: &Path) -> String {
    let source = std::fs::read_to_string(lib_rs)
        .unwrap_or_else(|e| panic!("read {}: {e}", lib_rs.display()));
    let file = match syn::parse_file(&source) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("anchor-asm: warning: can't parse {}: {e}", lib_rs.display());
            return String::new();
        }
    };

    let mut out = String::new();

    for item in &file.items {
        // Also check mod items for structs defined in submodules
        // that are inlined with `mod foo { ... }`.
        match item {
            syn::Item::Struct(s) if has_account_attr(s) => {
                if let Some(block) = emit_struct(s) {
                    out.push_str(&block);
                }
            }
            syn::Item::Mod(m) => {
                if let Some((_, items)) = &m.content {
                    for item in items {
                        if let syn::Item::Struct(s) = item {
                            if has_account_attr(s) {
                                if let Some(block) = emit_struct(s) {
                                    out.push_str(&block);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    out
}

/// Check if a struct should have assembly constants generated.
/// Matches `#[account]` (anchor v2) or `#[repr(C)]` (plain Pod).
fn has_account_attr(s: &syn::ItemStruct) -> bool {
    s.attrs.iter().any(|attr| {
        let path = attr.path();
        let last = path.segments.last().map(|s| s.ident.to_string());
        matches!(last.as_deref(), Some("account" | "repr"))
    })
}

/// Emit `.equ` constants for a single struct.
fn emit_struct(s: &syn::ItemStruct) -> Option<String> {
    let name = &s.ident;
    let fields = match &s.fields {
        syn::Fields::Named(f) => &f.named,
        _ => return None,
    };

    let mut out = String::new();
    out.push_str(&format!("# {name} field offsets and sizes.\n"));
    out.push_str(&format!(
        "# {}\n",
        "-".repeat(70)
    ));

    // Compute repr(C) layout: fields in declaration order, each aligned
    // to its natural alignment, struct padded to max alignment at end.
    let mut offset: usize = 0;
    let mut max_align: usize = 1;

    for field in fields {
        let field_name = field.ident.as_ref()?;

        // Skip fields starting with _ (padding).
        let name_str = field_name.to_string();
        if name_str.starts_with('_') {
            let (size, align) = type_layout(&field.ty)?;
            offset = align_up(offset, align);
            offset += size;
            if align > max_align {
                max_align = align;
            }
            continue;
        }

        let (size, align) = type_layout(&field.ty)?;
        offset = align_up(offset, align);

        out.push_str(&format!(".equ {name}__{field_name}, {offset}\n"));

        offset += size;
        if align > max_align {
            max_align = align;
        }
    }

    // Pad to struct alignment.
    let struct_size = align_up(offset, max_align);

    out.push_str(&format!(".equ {name}__SIZE, {struct_size}\n"));
    out.push_str(&format!(".equ {name}__DISC_SIZE, 8\n"));
    out.push_str(&format!(
        ".equ {name}__INIT_SPACE, {}\n",
        8 + struct_size
    ));
    out.push_str(&format!(
        "# {}\n\n",
        "-".repeat(70)
    ));

    Some(out)
}

/// Returns (size, alignment) for a type, matching `#[repr(C)]` / Pod layout.
/// Only handles the types that make sense in `#[account]` structs.
fn type_layout(ty: &syn::Type) -> Option<(usize, usize)> {
    match ty {
        syn::Type::Path(tp) => {
            let seg = tp.path.segments.last()?;
            let name = seg.ident.to_string();
            match name.as_str() {
                "u8" | "i8" | "bool" => Some((1, 1)),
                "u16" | "i16" => Some((2, 2)),
                "u32" | "i32" | "f32" => Some((4, 4)),
                "u64" | "i64" | "f64" => Some((8, 8)),
                "u128" | "i128" => Some((16, 16)),
                // Anchor v2 Address = [u8; 32], alignment 1
                "Address" | "Pubkey" => Some((32, 1)),
                // PodBool = u8
                "PodBool" => Some((1, 1)),
                // Pod wrappers — alignment 1, stored as [u8; N]
                "PodU16" | "PodI16" => Some((2, 1)),
                "PodU32" | "PodI32" => Some((4, 1)),
                "PodU64" | "PodI64" => Some((8, 1)),
                "PodU128" | "PodI128" => Some((16, 1)),
                // PodVec<T, MAX> — need to inspect generic args
                "PodVec" => pod_vec_layout(&seg.arguments),
                _ => None,
            }
        }
        syn::Type::Array(arr) => {
            let (elem_size, elem_align) = type_layout(&arr.elem)?;
            let len = array_len(&arr.len)?;
            Some((elem_size * len, elem_align))
        }
        _ => None,
    }
}

/// Compute layout for `PodVec<T, MAX>`: `[u16 len][T; MAX]` with alignment 1.
fn pod_vec_layout(args: &syn::PathArguments) -> Option<(usize, usize)> {
    let syn::PathArguments::AngleBracketed(ab) = args else {
        return None;
    };
    let mut iter = ab.args.iter();

    // First arg: element type
    let syn::GenericArgument::Type(elem_ty) = iter.next()? else {
        return None;
    };
    let (elem_size, _) = type_layout(elem_ty)?;

    // Second arg: MAX capacity (const generic)
    let max = match iter.next()? {
        syn::GenericArgument::Const(expr) => const_expr_value(expr)?,
        syn::GenericArgument::Type(syn::Type::Path(_)) => {
            // Could be a const path like MAX_SIGNERS — can't resolve,
            // skip this struct.
            return None;
        }
        _ => return None,
    };

    // PodVec layout: 2 bytes (u16 len) + elem_size * max
    Some((2 + elem_size * max, 1))
}

/// Extract a usize from a const expression (integer literal).
fn const_expr_value(expr: &syn::Expr) -> Option<usize> {
    if let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Int(i),
        ..
    }) = expr
    {
        i.base10_parse().ok()
    } else {
        None
    }
}

/// Extract array length from a const expression.
fn array_len(expr: &syn::Expr) -> Option<usize> {
    const_expr_value(expr)
}

fn align_up(offset: usize, align: usize) -> usize {
    (offset + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_struct() {
        let source = r#"
            #[account]
            pub struct Counter {
                pub value: u64,
                pub bump: u8,
                pub _pad: [u8; 7],
            }
        "#;
        let tmp = std::env::temp_dir().join("anchor_asm_test_lib.rs");
        std::fs::write(&tmp, source).unwrap();
        let result = generate(&tmp);
        assert!(result.contains(".equ Counter__value, 0"));
        assert!(result.contains(".equ Counter__bump, 8"));
        assert!(result.contains(".equ Counter__SIZE, 16"));
        assert!(result.contains(".equ Counter__INIT_SPACE, 24"));
        std::fs::remove_file(tmp).ok();
    }

    #[test]
    fn test_address_field() {
        let source = r#"
            #[account]
            pub struct Config {
                pub admin: Address,
                pub bump: u8,
            }
        "#;
        let tmp = std::env::temp_dir().join("anchor_asm_test_addr.rs");
        std::fs::write(&tmp, source).unwrap();
        let result = generate(&tmp);
        // Address is 32 bytes, align 1
        assert!(result.contains(".equ Config__admin, 0"));
        assert!(result.contains(".equ Config__bump, 32"));
        std::fs::remove_file(tmp).ok();
    }
}
