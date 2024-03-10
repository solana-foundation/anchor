use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

pub fn find_path(name: &str, path: impl AsRef<Path>) -> Result<PathBuf> {
    let parent_path = path.as_ref().parent().unwrap();
    for entry in fs::read_dir(parent_path)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy() == name {
            return entry.path().canonicalize().map_err(Into::into);
        }
    }

    find_path(name, parent_path)
}

pub fn get_no_docs() -> bool {
    option_env!("ANCHOR_IDL_BUILD_NO_DOCS")
        .map(|val| val == "TRUE")
        .unwrap_or_default()
}

pub fn get_idl_module_path() -> TokenStream {
    quote!(anchor_lang::anchor_syn::idl::types)
}

pub fn get_serde_json_module_path() -> TokenStream {
    quote!(anchor_lang::anchor_syn::idl::build::serde_json)
}

pub fn gen_print_section(name: &str, value: impl ToTokens) -> TokenStream {
    let serde_json = get_serde_json_module_path();
    quote! {
        println!("--- IDL begin {} ---", #name);
        println!("{}", #serde_json::to_string_pretty(&{ #value }).unwrap());
        println!("--- IDL end {} ---", #name);
    }
}
