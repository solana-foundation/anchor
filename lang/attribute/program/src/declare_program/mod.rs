mod common;
mod mods;

use anchor_lang_idl::types::Idl;
use anyhow::anyhow;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};

use common::gen_docs;
use mods::{
    accounts::gen_accounts_mod, client::gen_client_mod, constants::gen_constants_mod,
    cpi::gen_cpi_mod, events::gen_events_mod, internal::gen_internal_mod, program::gen_program_mod,
    types::gen_types_mod, utils::gen_utils_mod,
};

pub struct DeclareProgram {
    name: syn::Ident,
    idl: Idl,
}

impl Parse for DeclareProgram {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        let idl = get_idl(&name).map_err(|e| syn::Error::new(name.span(), e))?;
        Ok(Self { name, idl })
    }
}

impl ToTokens for DeclareProgram {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let program = gen_program(&self.idl, &self.name);
        tokens.extend(program)
    }
}

fn get_idl(name: &syn::Ident) -> anyhow::Result<Idl> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("Failed to get manifest dir");
    let path = std::path::Path::new(&manifest_dir)
        .ancestors()
        .find_map(|ancestor| {
            let idl_dir = ancestor.join("idls");
            std::fs::metadata(&idl_dir).map(|_| idl_dir).ok()
        })
        .ok_or_else(|| anyhow!("`idls` directory not found"))
        .map(|idl_dir| idl_dir.join(name.to_string()).with_extension("json"))?;

    std::fs::read(path)
        .map_err(|e| anyhow!("Failed to read IDL: {e}"))
        .map(|idl| serde_json::from_slice(&idl))?
        .map_err(|e| anyhow!("Failed to parse IDL: {e}"))
}

fn gen_program(idl: &Idl, name: &syn::Ident) -> proc_macro2::TokenStream {
    let docs = gen_program_docs(idl);
    let id = gen_id(idl);
    let program_mod = gen_program_mod(&idl.metadata.name);

    // Defined
    let constants_mod = gen_constants_mod(idl);
    let accounts_mod = gen_accounts_mod(idl);
    let events_mod = gen_events_mod(idl);
    let types_mod = gen_types_mod(idl);

    // Clients
    let cpi_mod = gen_cpi_mod(idl);
    let client_mod = gen_client_mod(idl);
    let internal_mod = gen_internal_mod(idl);

    // Utils
    let utils_mod = gen_utils_mod(idl);

    quote! {
        #docs
        pub mod #name {
            use anchor_lang::prelude::*;

            #id
            #program_mod

            #constants_mod
            #accounts_mod
            #events_mod
            #types_mod

            #cpi_mod
            #client_mod
            #internal_mod

            #utils_mod
        }
    }
}

fn gen_program_docs(idl: &Idl) -> proc_macro2::TokenStream {
    let docs: &[String] = &[
        format!(
            "Generated external program declaration of program `{}`.",
            idl.metadata.name
        ),
        String::default(),
    ];
    let docs = [docs, &idl.docs].concat();
    gen_docs(&docs)
}

fn gen_id(idl: &Idl) -> proc_macro2::TokenStream {
    let address_bytes = bs58::decode(&idl.address)
        .into_vec()
        .expect("Invalid `idl.address`");
    let doc = format!("Program ID of program `{}`.", idl.metadata.name);

    quote! {
        #[doc = #doc]
        pub static ID: Pubkey = __ID;

        /// The name is intentionally prefixed with `__` in order to reduce to possibility of name
        /// clashes with the crate's `ID`.
        static __ID: Pubkey = Pubkey::new_from_array([#(#address_bytes,)*]);
    }
}
