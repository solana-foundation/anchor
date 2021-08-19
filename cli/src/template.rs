use crate::config::ProgramWorkspace;
use crate::VERSION;
use anyhow::Result;
use heck::{CamelCase, SnakeCase};

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
        r#"
import * as anchor from '@project-serum/anchor';

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
    r#"
// Migrations are an early feature. Currently, they're nothing more than this
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
    r#"
// Migrations are an early feature. Currently, they're nothing more than this
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

const __wallet = new anchor.Wallet(
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
            program.program_id.to_string()
        ));
    }

    Ok(eval_string)
}
