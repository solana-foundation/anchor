use crate::parser::reload_check;
use crate::Program;
use quote::quote;
use std::env;
use std::path::PathBuf;

mod accounts;
pub mod common;
mod cpi;
mod dispatch;
mod entry;
mod handlers;
mod idl;
mod instruction;

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    let mod_name = &program.name;

    // Try to get file-level items by reading the source file
    let file_items = try_read_source_file(program);

    // Check for missing reload() after CPI
    let violations = if !file_items.is_empty() {
        // Use file-level checking if we successfully read the file
        reload_check::check_program_with_file_items(program, &file_items)
    } else {
        // Fall back to module-only checking
        reload_check::check_program(program)
    };

    if !violations.is_empty() {
        // Combine all errors into a single syn::Error with multiple spans
        let mut combined_error = None;
        for violation in &violations {
            let error = violation.to_error();
            combined_error = match combined_error {
                None => Some(error),
                Some(mut prev) => {
                    prev.combine(error);
                    Some(prev)
                }
            };
        }

        // Return the error directly as token stream
        if let Some(err) = combined_error {
            return err.to_compile_error();
        }
    }

    let entry = entry::generate(program);
    let dispatch = dispatch::generate(program);
    let handlers = handlers::generate(program);
    let user_defined_program = &program.program_mod;
    let instruction = instruction::generate(program);
    let cpi = cpi::generate(program);
    let accounts = accounts::generate(program);

    #[allow(clippy::let_and_return)]
    let ret = {
        quote! {
            // TODO: remove once we allow segmented paths in `Accounts` structs.
            use self::#mod_name::*;

            #entry
            #dispatch
            #handlers
            #user_defined_program
            #instruction
            #cpi
            #accounts
        }
    };

    #[cfg(feature = "idl-build")]
    {
        let idl_build_impl = crate::idl::gen_idl_print_fn_program(program);
        return quote! {
            #ret
            #idl_build_impl
        };
    };

    #[allow(unreachable_code)]
    ret
}

/// Try to read ALL source files to get file-level items (impl blocks outside #[program])
/// This enables checking of code defined outside the #[program] module
/// Recursively scans all .rs files in the src/ directory
fn try_read_source_file(_program: &Program) -> Vec<syn::Item> {
    // Try to find and parse ALL rust files in the source directory
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = PathBuf::from(manifest_dir);
        let src_dir = manifest_path.join("src");

        if src_dir.exists() && src_dir.is_dir() {
            // Recursively collect all .rs files
            if let Ok(items) = collect_items_from_dir(&src_dir) {
                return items;
            }
        }
    }

    // Couldn't read files, return empty vec (will fall back to module-only checking)
    Vec::new()
}

/// Recursively collect syn::Items from all .rs files in a directory
fn collect_items_from_dir(dir: &PathBuf) -> std::io::Result<Vec<syn::Item>> {
    use std::fs;

    let mut items = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recurse into subdirectories
            if let Ok(sub_items) = collect_items_from_dir(&path) {
                items.extend(sub_items);
            }
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            // Parse .rs file
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(file) = syn::parse_file(&content) {
                    items.extend(file.items);
                }
            }
        }
    }

    Ok(items)
}
