use crate::parser;
use crate::{IxArg, State};
use heck::CamelCase;
use quote::quote;

// Namespace for calculating state instruction sighash signatures.
pub const SIGHASH_STATE_NAMESPACE: &str = "state";

// Namespace for calculating instruction sighash signatures for any instruction
// not affecting program state.
pub const SIGHASH_GLOBAL_NAMESPACE: &str = "global";

// We don't technically use sighash, because the input arguments aren't given.
// Rust doesn't have method overloading so no need to use the arguments.
// However, we do namespace methods in the preeimage so that we can use
// different traits with the same method name.
pub fn sighash(namespace: &str, name: &str) -> [u8; 8] {
    let preimage = format!("{}:{}", namespace, name);

    let mut sighash = [0u8; 8];
    sighash.copy_from_slice(&crate::hash::hash(preimage.as_bytes()).to_bytes()[..8]);
    sighash
}

pub fn sighash_ctor() -> [u8; 8] {
    sighash(SIGHASH_STATE_NAMESPACE, "new")
}

pub fn generate_ix_variant(name: String, args: &[IxArg]) -> proc_macro2::TokenStream {
    let ix_arg_names: Vec<&syn::Ident> = args.iter().map(|arg| &arg.name).collect();
    let ix_name_camel: proc_macro2::TokenStream = {
        let n = name.to_camel_case();
        n.parse().unwrap()
    };

    if args.is_empty() {
        quote! {
            #ix_name_camel
        }
    } else {
        quote! {
            #ix_name_camel {
                #(#ix_arg_names),*
            }
        }
    }
}

pub fn generate_ctor_args(state: &State) -> Vec<syn::Pat> {
    generate_ctor_typed_args(state)
        .iter()
        .map(|pat_ty| *pat_ty.pat.clone())
        .collect()
}

pub fn generate_ctor_typed_args(state: &State) -> Vec<syn::PatType> {
    state
        .ctor_and_anchor
        .as_ref()
        .map(|(ctor, _anchor_ident)| {
            ctor.sig
                .inputs
                .iter()
                .filter_map(|arg: &syn::FnArg| match arg {
                    syn::FnArg::Typed(pat_ty) => {
                        let mut arg_str = parser::tts_to_string(&pat_ty.ty);
                        arg_str.retain(|c| !c.is_whitespace());
                        if arg_str.starts_with("Context<") {
                            return None;
                        }
                        Some(pat_ty.clone())
                    }
                    _ => {
                        if !state.is_zero_copy {
                            panic!("Cannot pass self as parameter")
                        }
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}
