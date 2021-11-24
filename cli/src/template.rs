use crate::config::ProgramWorkspace;
use crate::VERSION;
use anchor_syn::idl::Idl;
use anyhow::Result;
use heck::{CamelCase, MixedCase, SnakeCase};
use solana_sdk::pubkey::Pubkey;

pub fn default_program_id() -> Pubkey {
    "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
        .parse()
        .unwrap()
}

pub fn virtual_manifest() -> &'static str {
    r#"[workspace]
members = [
    "programs/*"
]
"#
}

pub fn credentials(token: &str) -> String {
    format!(
        r#"[registry]
token = "{}"
"#,
        token
    )
}

pub fn idl_ts(idl: &Idl) -> Result<String> {
    let mut idl = idl.clone();
    for acc in idl.accounts.iter_mut() {
        acc.name = acc.name.to_mixed_case();
    }
    let idl_json = serde_json::to_string_pretty(&idl)?;
    Ok(format!(
        r#"export type {} = {};

export const IDL: {} = {};
"#,
        idl.name.to_camel_case(),
        idl_json,
        idl.name.to_camel_case(),
        idl_json
    ))
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
no-idl = []
no-log-ix-name = []
cpi = ["no-entrypoint"]
default = []

[dependencies]
anchor-lang = "{2}"
"#,
        name,
        name.to_snake_case(),
        VERSION,
    )
}

pub fn deploy_js_script_host(cluster_url: &str, script_path: &str) -> String {
    format!(
        r#"
const anchor = require('@project-serum/anchor');

// Deploy script defined by the user.
const userScript = require("{0}");

async function main() {{
    const url = "{1}";
    const preflightCommitment = 'recent';
    const connection = new anchor.web3.Connection(url, preflightCommitment);
    const wallet = anchor.Wallet.local();

    const provider = new anchor.Provider(connection, wallet, {{
        preflightCommitment,
        commitment: 'recent',
    }});

    // Run the user's deploy script.
    userScript(provider);
}}
main();
"#,
        script_path, cluster_url,
    )
}

pub fn deploy_ts_script_host(cluster_url: &str, script_path: &str) -> String {
    format!(
        r#"import * as anchor from '@project-serum/anchor';

// Deploy script defined by the user.
const userScript = require("{0}");

async function main() {{
    const url = "{1}";
    const preflightCommitment = 'recent';
    const connection = new anchor.web3.Connection(url, preflightCommitment);
    const wallet = anchor.Wallet.local();

    const provider = new anchor.Provider(connection, wallet, {{
        preflightCommitment,
        commitment: 'recent',
    }});

    // Run the user's deploy script.
    userScript(provider);
}}
main();
"#,
        script_path, cluster_url,
    )
}

pub fn deploy_script() -> &'static str {
    r#"// Migrations are an early feature. Currently, they're nothing more than this
// single deploy script that's invoked from the CLI, injecting a provider
// configured from the workspace's Anchor.toml.

const anchor = require("@project-serum/anchor");

module.exports = async function (provider) {
  // Configure client to use the provider.
  anchor.setProvider(provider);

  // Add your deploy script here.
}
"#
}

pub fn ts_deploy_script() -> &'static str {
    r#"// Migrations are an early feature. Currently, they're nothing more than this
// single deploy script that's invoked from the CLI, injecting a provider
// configured from the workspace's Anchor.toml.

const anchor = require("@project-serum/anchor");

module.exports = async function (provider) {
  // Configure client to use the provider.
  anchor.setProvider(provider);

  // Add your deploy script here.
}
"#
}

pub fn xargo_toml() -> &'static str {
    r#"[target.bpfel-unknown-unknown.dependencies.std]
features = []
"#
}

pub fn lib_rs(name: &str) -> String {
    format!(
        r#"use anchor_lang::prelude::*;

declare_id!("{}");

#[program]
pub mod {} {{
    use super::*;
    pub fn initialize(ctx: Context<Initialize>) -> ProgramResult {{
        Ok(())
    }}
}}

#[derive(Accounts)]
pub struct Initialize {{}}
"#,
        default_program_id(),
        name.to_snake_case(),
    )
}

pub fn mocha(name: &str) -> String {
    format!(
        r#"const anchor = require('@project-serum/anchor');

describe('{}', () => {{

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

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

pub fn package_json() -> String {
    format!(
        r#"{{
    "dependencies": {{
        "@project-serum/anchor": "^{0}"
    }},
    "devDependencies": {{
        "chai": "^4.3.4",
        "mocha": "^9.0.3"
    }}
}}
"#,
        VERSION
    )
}

pub fn ts_package_json() -> String {
    format!(
        r#"{{
    "dependencies": {{
        "@project-serum/anchor": "^{0}"
    }},
    "devDependencies": {{
        "chai": "^4.3.4",
        "mocha": "^9.0.3",
        "ts-mocha": "^8.0.0",
        "@types/mocha": "^9.0.0",
        "typescript": "^4.3.5"
    }}
}}
"#,
        VERSION
    )
}

pub fn ts_mocha(name: &str) -> String {
    format!(
        r#"import * as anchor from '@project-serum/anchor';
import {{ Program }} from '@project-serum/anchor';
import {{ {} }} from '../target/types/{}';

describe('{}', () => {{

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.{} as Program<{}>;

  it('Is initialized!', async () => {{
    // Add your test here.
    const tx = await program.rpc.initialize({{}});
    console.log("Your transaction signature", tx);
  }});
}});
"#,
        name.to_camel_case(),
        name.to_snake_case(),
        name,
        name.to_camel_case(),
        name.to_camel_case(),
    )
}

pub fn ts_config() -> &'static str {
    r#"{
  "compilerOptions": {
    "types": ["mocha", "chai"],
    "typeRoots": ["./node_modules/@types"],
    "lib": ["es2015"],
    "module": "commonjs",
    "target": "es6",
    "esModuleInterop": true
  }
}
"#
}

pub fn git_ignore() -> &'static str {
    r#"
.anchor
.DS_Store
target
**/*.rs.bk
node_modules
"#
}

pub fn node_shell(
    cluster_url: &str,
    wallet_path: &str,
    programs: Vec<ProgramWorkspace>,
) -> Result<String> {
    let mut eval_string = format!(
        r#"
const anchor = require('@project-serum/anchor');
const web3 = anchor.web3;
const PublicKey = anchor.web3.PublicKey;
const Keypair = anchor.web3.Keypair;

const __wallet = new anchor.Wallet(
  Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        require('fs').readFileSync(
          "{}",
          {{
            encoding: "utf-8",
          }},
        ),
      ),
    ),
  ),
);
const __connection = new web3.Connection("{}", "processed");
const provider = new anchor.Provider(__connection, __wallet, {{
  commitment: "processed",
  preflightcommitment: "processed",
}});
anchor.setProvider(provider);
"#,
        wallet_path, cluster_url,
    );

    for program in programs {
        eval_string.push_str(&format!(
            r#"
anchor.workspace.{} = new anchor.Program({}, new PublicKey("{}"), provider);
"#,
            program.name.to_camel_case(),
            serde_json::to_string(&program.idl)?,
            program.program_id
        ));
    }

    Ok(eval_string)
}
