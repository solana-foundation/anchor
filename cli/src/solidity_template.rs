use crate::config::ProgramWorkspace;
use crate::VERSION;
use anchor_syn::idl::Idl;
use anyhow::Result;
use heck::{ToLowerCamelCase, ToSnakeCase, ToUpperCamelCase};
use solana_sdk::pubkey::Pubkey;
use std::fmt::Write;

pub fn default_program_id() -> Pubkey {
    "F1ipperKF9EfD821ZbbYjS319LXYiBmjhzkkf5a26rC"
        .parse()
        .unwrap()
}

pub fn idl_ts(idl: &Idl) -> Result<String> {
    let mut idl = idl.clone();
    for acc in idl.accounts.iter_mut() {
        acc.name = acc.name.to_lower_camel_case();
    }
    let idl_json = serde_json::to_string_pretty(&idl)?;
    Ok(format!(
        r#"export type {} = {};

export const IDL: {} = {};
"#,
        idl.name.to_upper_camel_case(),
        idl_json,
        idl.name.to_upper_camel_case(),
        idl_json
    ))
}

pub fn deploy_js_script_host(cluster_url: &str, script_path: &str) -> String {
    format!(
        r#"
const anchor = require('@coral-xyz/anchor');

// Deploy script defined by the user.
const userScript = require("{script_path}");

async function main() {{
    const url = "{cluster_url}";
    const preflightCommitment = 'recent';
    const connection = new anchor.web3.Connection(url, preflightCommitment);
    const wallet = anchor.Wallet.local();

    const provider = new anchor.AnchorProvider(connection, wallet, {{
        preflightCommitment,
        commitment: 'recent',
    }});

    // Run the user's deploy script.
    userScript(provider);
}}
main();
"#
    )
}

pub fn deploy_ts_script_host(cluster_url: &str, script_path: &str) -> String {
    format!(
        r#"import * as anchor from '@coral-xyz/anchor';

// Deploy script defined by the user.
const userScript = require("{script_path}");

async function main() {{
    const url = "{cluster_url}";
    const preflightCommitment = 'recent';
    const connection = new anchor.web3.Connection(url, preflightCommitment);
    const wallet = anchor.Wallet.local();

    const provider = new anchor.AnchorProvider(connection, wallet, {{
        preflightCommitment,
        commitment: 'recent',
    }});

    // Run the user's deploy script.
    userScript(provider);
}}
main();
"#
    )
}

pub fn deploy_script() -> &'static str {
    r#"// Migrations are an early feature. Currently, they're nothing more than this
// single deploy script that's invoked from the CLI, injecting a provider
// configured from the workspace's Anchor.toml.

const anchor = require("@coral-xyz/anchor");

module.exports = async function (provider) {
  // Configure client to use the provider.
  anchor.setProvider(provider);

  // Add your deploy script here.
};
"#
}

pub fn ts_deploy_script() -> &'static str {
    r#"// Migrations are an early feature. Currently, they're nothing more than this
// single deploy script that's invoked from the CLI, injecting a provider
// configured from the workspace's Anchor.toml.

const anchor = require("@coral-xyz/anchor");

module.exports = async function (provider) {
  // Configure client to use the provider.
  anchor.setProvider(provider);

  // Add your deploy script here.
};
"#
}

pub fn solidity(name: &str) -> String {
    format!(
        r#"
@program_id("{}")
contract {} {{
    bool private value = true;

    @payer(payer)
    constructor(address payer) {{
        print("Hello, World!");
    }}

    /// A message that can be called on instantiated contracts.
    /// This one flips the value of the stored `bool` from `true`
    /// to `false` and vice versa.
    function flip() public {{
            value = !value;
    }}

    /// Simply returns the current value of our `bool`.
    function get() public view returns (bool) {{
            return value;
    }}
}}
"#,
        default_program_id(),
        name.to_snake_case(),
    )
}

pub fn mocha(name: &str) -> String {
    format!(
        r#"const anchor = require("@coral-xyz/anchor");

describe("{}", () => {{
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  it("Is initialized!", async () => {{
    // Add your test here.
    const program = anchor.workspace.{};
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);

    const val1 = await program.methods.get()
      .accounts({{ dataAccount: dataAccount.publicKey }})
      .view();

    console.log("state", val1);

    await program.methods.flip()
      .accounts({{ dataAccount: dataAccount.publicKey }})
      .rpc();

    const val2 = await program.methods.get()
      .accounts({{ dataAccount: dataAccount.publicKey }})
      .view();

    console.log("state", val2);
  }});
}});
"#,
        name,
        name.to_upper_camel_case(),
    )
}

pub fn jest(name: &str) -> String {
    format!(
        r#"const anchor = require("@coral-xyz/anchor");

describe("{}", () => {{
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  it("Is initialized!", async () => {{
    // Add your test here.
    const program = anchor.workspace.{};
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  }});
}});
"#,
        name,
        name.to_upper_camel_case(),
    )
}

pub fn package_json(jest: bool) -> String {
    if jest {
        format!(
            r#"{{
        "scripts": {{
            "lint:fix": "prettier */*.js \"*/**/*{{.js,.ts}}\" -w",
            "lint": "prettier */*.js \"*/**/*{{.js,.ts}}\" --check"
        }},
        "dependencies": {{
            "@coral-xyz/anchor": "^{VERSION}"
        }},
        "devDependencies": {{
            "jest": "^29.0.3",
            "prettier": "^2.6.2"
        }}
    }}
    "#
        )
    } else {
        format!(
            r#"{{
    "scripts": {{
        "lint:fix": "prettier */*.js \"*/**/*{{.js,.ts}}\" -w",
        "lint": "prettier */*.js \"*/**/*{{.js,.ts}}\" --check"
    }},
    "dependencies": {{
        "@coral-xyz/anchor": "^{VERSION}"
    }},
    "devDependencies": {{
        "chai": "^4.3.4",
        "mocha": "^9.0.3",
        "prettier": "^2.6.2"
    }}
}}
"#
        )
    }
}

pub fn ts_package_json(jest: bool) -> String {
    if jest {
        format!(
            r#"{{
        "scripts": {{
            "lint:fix": "prettier */*.js \"*/**/*{{.js,.ts}}\" -w",
            "lint": "prettier */*.js \"*/**/*{{.js,.ts}}\" --check"
        }},
        "dependencies": {{
            "@coral-xyz/anchor": "^{VERSION}"
        }},
        "devDependencies": {{
            "@types/bn.js": "^5.1.0",
            "@types/jest": "^29.0.3",
            "jest": "^29.0.3",
            "prettier": "^2.6.2",
            "ts-jest": "^29.0.2",
            "typescript": "^4.3.5"
        }}
    }}
    "#
        )
    } else {
        format!(
            r#"{{
    "scripts": {{
        "lint:fix": "prettier */*.js \"*/**/*{{.js,.ts}}\" -w",
        "lint": "prettier */*.js \"*/**/*{{.js,.ts}}\" --check"
    }},
    "dependencies": {{
        "@coral-xyz/anchor": "^{VERSION}"
    }},
    "devDependencies": {{
        "chai": "^4.3.4",
        "mocha": "^9.0.3",
        "ts-mocha": "^10.0.0",
        "@types/bn.js": "^5.1.0",
        "@types/chai": "^4.3.0",
        "@types/mocha": "^9.0.0",
        "typescript": "^4.3.5",
        "prettier": "^2.6.2"
    }}
}}
"#
        )
    }
}

pub fn ts_mocha(name: &str) -> String {
    format!(
        r#"import * as anchor from "@coral-xyz/anchor";
import {{ Program }} from "@coral-xyz/anchor";
import {{ {} }} from "../target/types/{}";

describe("{}", () => {{
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const dataAccount = anchor.web3.Keypair.generate();
  const wallet = provider.wallet;

  const program = anchor.workspace.{} as Program<{}>;

  it("Is initialized!", async () => {{
    // Add your test here.
    const tx = await program.methods.new(wallet.publicKey)
      .accounts({{ dataAccount: dataAccount.publicKey }})
      .signers([dataAccount]).rpc();
    console.log("Your transaction signature", tx);

    const val1 = await program.methods.get()
      .accounts({{ dataAccount: dataAccount.publicKey }})
      .view();

    console.log("state", val1);

    await program.methods.flip()
      .accounts({{ dataAccount: dataAccount.publicKey }})
      .rpc();

    const val2 = await program.methods.get()
      .accounts({{ dataAccount: dataAccount.publicKey }})
      .view();

    console.log("state", val2);  }});
}});
"#,
        name.to_upper_camel_case(),
        name.to_snake_case(),
        name,
        name.to_upper_camel_case(),
        name.to_upper_camel_case(),
    )
}

pub fn ts_jest(name: &str) -> String {
    format!(
        r#"import * as anchor from "@coral-xyz/anchor";
import {{ Program }} from "@coral-xyz/anchor";
import {{ {} }} from "../target/types/{}";

describe("{}", () => {{
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const dataAccount = anchor.web3.Keypair.generate();
  const wallet = provider.wallet;

  const program = anchor.workspace.{} as Program<{}>;

  it("Is initialized!", async () => {{
    // Add your test here.
    const tx = await program.methods.new(wallet.publicKey)
      .accounts({{ dataAccount: dataAccount.publicKey }})
      .signers([dataAccount]).rpc();
    console.log("Your transaction signature", tx);
  }});
}});
"#,
        name.to_upper_camel_case(),
        name.to_snake_case(),
        name,
        name.to_upper_camel_case(),
        name.to_upper_camel_case(),
    )
}

pub fn ts_config(jest: bool) -> &'static str {
    if jest {
        r#"{
            "compilerOptions": {
              "types": ["jest"],
              "typeRoots": ["./node_modules/@types"],
              "lib": ["es2015"],
              "module": "commonjs",
              "target": "es6",
              "esModuleInterop": true
            }
          }
          "#
    } else {
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
}

pub fn git_ignore() -> &'static str {
    r#"
.anchor
.DS_Store
target
**/*.rs.bk
node_modules
test-ledger
"#
}

pub fn prettier_ignore() -> &'static str {
    r#"
.anchor
.DS_Store
target
node_modules
dist
build
test-ledger
"#
}

pub fn node_shell(
    cluster_url: &str,
    wallet_path: &str,
    programs: Vec<ProgramWorkspace>,
) -> Result<String> {
    let mut eval_string = format!(
        r#"
const anchor = require('@coral-xyz/anchor');
const web3 = anchor.web3;
const PublicKey = anchor.web3.PublicKey;
const Keypair = anchor.web3.Keypair;

const __wallet = new anchor.Wallet(
  Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        require('fs').readFileSync(
          "{wallet_path}",
          {{
            encoding: "utf-8",
          }},
        ),
      ),
    ),
  ),
);
const __connection = new web3.Connection("{cluster_url}", "processed");
const provider = new anchor.AnchorProvider(__connection, __wallet, {{
  commitment: "processed",
  preflightcommitment: "processed",
}});
anchor.setProvider(provider);
"#
    );

    for program in programs {
        write!(
            &mut eval_string,
            r#"
anchor.workspace.{} = new anchor.Program({}, new PublicKey("{}"), provider);
"#,
            program.name.to_upper_camel_case(),
            serde_json::to_string(&program.idl)?,
            program.program_id
        )?;
    }

    Ok(eval_string)
}
