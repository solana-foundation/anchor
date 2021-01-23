use heck::CamelCase;
use heck::SnakeCase;

pub fn virtual_manifest() -> String {
    r#"[workspace]
members = [
    "programs/*"
]
"#
    .to_string()
}

pub fn cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{0}"
version = "0.1.0"
description = "Created with Anchor"
edition = "2018"

[lib]
crate-type = ["cdylib", "lib"]
name = "{1}"

[features]
no-entrypoint = []
cpi = ["no-entrypoint"]

[dependencies]
anchor-lang = {{ git = "https://github.com/project-serum/anchor", features = ["derive"] }}
"#,
        name,
        name.to_snake_case(),
    )
}

pub fn xargo_toml() -> String {
    r#"[target.bpfel-unknown-unknown.dependencies.std]
features = []"#
        .to_string()
}

pub fn lib_rs(name: &str) -> String {
    format!(
        r#"#![feature(proc_macro_hygiene)]

use anchor_lang::prelude::*;

#[program]
pub mod {} {{
    use super::*;
    pub fn initialize(ctx: Context<Initialize>) -> ProgramResult {{
        Ok(())
    }}
}}

#[derive(Accounts)]
pub struct Initialize {{}}"#,
        name.to_snake_case(),
    )
}

pub fn mocha(name: &str) -> String {
    format!(
        r#"const anchor = require('@project-serum/anchor');

describe('{}', () => {{

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.local());

  it('Is initialized!', async () => {{
    // Add your test here.
    const program = anchor.workspace.{};
    const tx = await program.rpc.initialize();
    console.log("Your transaction signature", tx);
  }});
}});
"#,
        name,
        name.to_camel_case(),
    )
}
