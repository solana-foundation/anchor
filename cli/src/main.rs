use crate::config::{find_cargo_toml, read_all_programs, Config, Program};
use anchor_syn::idl::Idl;
use anyhow::{anyhow, Result};
use clap::Clap;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::string::ToString;

mod config;
mod template;

#[derive(Debug, Clap)]
pub struct Opts {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clap)]
pub enum Command {
    /// Initializes a workspace.
    Init { name: String },
    /// Builds a Solana program.
    Build {
        /// Output directory for the IDL.
        #[clap(short, long)]
        idl: Option<String>,
    },
    /// Runs integration tests against a localnetwork.
    Test,
    /// Creates a new program.
    New { name: String },
    /// Outputs an interface definition file.
    Idl {
        /// Path to the program's interface definition.
        #[clap(short, long)]
        file: String,
        /// Output file for the idl (stdout if not specified).
        #[clap(short, long)]
        out: Option<String>,
    },
    /// Deploys the workspace to the configured cluster.
    Deploy {
        #[clap(short, long)]
        url: Option<String>,
        #[clap(short, long)]
        keypair: Option<String>,
    },
}

fn main() -> Result<()> {
    let opts = Opts::parse();

    match opts.command {
        Command::Init { name } => init(name),
        Command::Build { idl } => build(idl),
        Command::Test => test(),
        Command::New { name } => new(name),
        Command::Idl { file, out } => {
            if out.is_none() {
                return idl(file, None);
            }
            idl(file, Some(&PathBuf::from(out.unwrap())))
        }
        Command::Deploy { url, keypair } => deploy(url, keypair),
    }
}

fn init(name: String) -> Result<()> {
    let cfg = Config::discover()?;

    if cfg.is_some() {
        println!("Anchor workspace already initialized");
    }

    fs::create_dir(name.clone())?;
    std::env::set_current_dir(&name)?;
    fs::create_dir("app")?;

    let cfg = Config::default();
    let toml = cfg.to_string();
    let mut file = File::create("Anchor.toml")?;
    file.write_all(toml.as_bytes())?;

    // Build virtual manifest.
    let mut virt_manifest = File::create("Cargo.toml")?;
    virt_manifest.write_all(template::virtual_manifest().as_bytes())?;

    // Build the program.
    fs::create_dir("programs")?;

    new_program(&name)?;

    // Build the test suite.
    fs::create_dir("tests")?;
    let mut mocha = File::create(&format!("tests/{}.js", name))?;
    mocha.write_all(template::mocha(&name).as_bytes())?;

    // Build the migrations directory.
    fs::create_dir("migrations")?;
    let mut deploy = File::create("migrations/deploy.js")?;
    deploy.write_all(&template::deploy_script().as_bytes())?;

    println!("{} initialized", name);

    Ok(())
}

// Creates a new program crate in the `programs/<name>` directory.
fn new(name: String) -> Result<()> {
    match Config::discover()? {
        None => {
            println!("Not in anchor workspace.");
            std::process::exit(1);
        }
        Some((_cfg, cfg_path, _inside_cargo)) => {
            match cfg_path.parent() {
                None => {
                    println!("Unable to make new program");
                }
                Some(parent) => {
                    std::env::set_current_dir(&parent)?;
                    new_program(&name)?;
                    println!("Created new program.");
                }
            };
        }
    }
    Ok(())
}

// Creates a new program crate in the current directory with `name`.
fn new_program(name: &str) -> Result<()> {
    fs::create_dir(&format!("programs/{}", name))?;
    fs::create_dir(&format!("programs/{}/src/", name))?;
    let mut cargo_toml = File::create(&format!("programs/{}/Cargo.toml", name))?;
    cargo_toml.write_all(template::cargo_toml(&name).as_bytes())?;
    let mut xargo_toml = File::create(&format!("programs/{}/Xargo.toml", name))?;
    xargo_toml.write_all(template::xargo_toml().as_bytes())?;
    let mut lib_rs = File::create(&format!("programs/{}/src/lib.rs", name))?;
    lib_rs.write_all(template::lib_rs(&name).as_bytes())?;
    Ok(())
}

fn build(idl: Option<String>) -> Result<()> {
    match Config::discover()? {
        None => build_cwd(idl),
        Some((cfg, cfg_path, inside_cargo)) => build_ws(cfg, cfg_path, inside_cargo, idl),
    }
}

// Runs the build inside a workspace.
//
// * Builds a single program if the current dir is within a Cargo subdirectory,
//   e.g., `programs/my-program/src`.
// * Builds *all* programs if thje current dir is anywhere else in the workspace.
//
fn build_ws(
    cfg: Config,
    cfg_path: PathBuf,
    cargo_toml: Option<PathBuf>,
    idl: Option<String>,
) -> Result<()> {
    let idl_out = match idl {
        Some(idl) => Some(PathBuf::from(idl)),
        None => {
            let cfg_parent = match cfg_path.parent() {
                None => return Err(anyhow::anyhow!("Invalid Anchor.toml")),
                Some(parent) => parent,
            };
            fs::create_dir_all(cfg_parent.join("target/idl"))?;
            Some(cfg_parent.join("target/idl"))
        }
    };
    match cargo_toml {
        None => build_all(cfg, cfg_path, idl_out),
        Some(ct) => _build_cwd(ct, idl_out),
    }
}

fn build_all(_cfg: Config, cfg_path: PathBuf, idl_out: Option<PathBuf>) -> Result<()> {
    match cfg_path.parent() {
        None => Err(anyhow::anyhow!(
            "Invalid Anchor.toml at {}",
            cfg_path.display()
        )),
        Some(parent) => {
            let files = fs::read_dir(parent.join("programs"))?;
            for f in files {
                let p = f?.path();
                _build_cwd(p.join("Cargo.toml"), idl_out.clone())?;
            }
            Ok(())
        }
    }
}

fn build_cwd(idl_out: Option<String>) -> Result<()> {
    match find_cargo_toml()? {
        None => {
            println!("Cargo.toml not found");
            std::process::exit(1);
        }
        Some(cargo_toml) => _build_cwd(cargo_toml, idl_out.map(PathBuf::from)),
    }
}

// Runs the build command outside of a workspace.
fn _build_cwd(cargo_toml: PathBuf, idl_out: Option<PathBuf>) -> Result<()> {
    match cargo_toml.parent() {
        None => return Err(anyhow::anyhow!("Unable to find parent")),
        Some(p) => std::env::set_current_dir(&p)?,
    };

    let exit = std::process::Command::new("cargo")
        .arg("build-bpf")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;
    if !exit.status.success() {
        std::process::exit(exit.status.code().unwrap_or(1));
    }

    // Always assume idl is located ar src/lib.rs.
    let idl = extract_idl("src/lib.rs")?;

    let out = match idl_out {
        None => PathBuf::from(".").join(&idl.name).with_extension("json"),
        Some(o) => PathBuf::from(&o.join(&idl.name).with_extension("json")),
    };

    write_idl(&idl, Some(&out))
}

fn idl(file: String, out: Option<&Path>) -> Result<()> {
    let idl = extract_idl(&file)?;
    write_idl(&idl, out)
}

fn extract_idl(file: &str) -> Result<Idl> {
    let file = shellexpand::tilde(file);
    anchor_syn::parser::file::parse(&*file)
}

fn write_idl(idl: &Idl, out: Option<&Path>) -> Result<()> {
    let idl_json = serde_json::to_string_pretty(idl)?;
    match out.as_ref() {
        None => println!("{}", idl_json),
        Some(out) => std::fs::write(out, idl_json)?,
    };
    Ok(())
}

// Builds, deploys, and tests all workspace programs in a single command.
fn test() -> Result<()> {
    // Switch directories to top level workspace.
    set_workspace_dir_or_exit();

    // Build everything.
    build(None)?;

    // Switch again (todo: restore cwd in `build` command).
    set_workspace_dir_or_exit();

    // Deploy all programs.
    let cfg = Config::discover()?.expect("Inside a workspace").0;

    // Bootup validator.
    let validator_handle = match cfg.cluster.url() {
        "http://127.0.0.1:8899" => Some(start_test_validator()?),
        _ => None,
    };

    let programs = deploy_ws(cfg.cluster.url(), &cfg.wallet.to_string())?;

    // Store deployed program addresses in IDL metadata (for consumption by
    // client + tests).
    for (program, address) in programs {
        // Add metadata to the IDL.
        let mut idl = program.idl;
        idl.metadata = Some(serde_json::to_value(IdlTestMetadata {
            address: address.to_string(),
        })?);
        // Persist it.
        let idl_out = PathBuf::from("target/idl")
            .join(&idl.name)
            .with_extension("json");
        write_idl(&idl, Some(&idl_out))?;
    }

    // Run the tests.
    if let Err(e) = std::process::Command::new("mocha")
        .arg("-t")
        .arg("10000")
        .arg("tests/")
        .env("ANCHOR_PROVIDER_URL", cfg.cluster.url())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
    {
        if let Some(mut validator_handle) = validator_handle {
            validator_handle.kill()?;
        }
        return Err(anyhow::format_err!("{}", e.to_string()));
    }
    if let Some(mut validator_handle) = validator_handle {
        validator_handle.kill()?;
    }

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdlTestMetadata {
    address: String,
}

fn start_test_validator() -> Result<Child> {
    fs::create_dir_all(".anchor")?;
    let test_ledger_filename = ".anchor/test-ledger";
    let test_ledger_log_filename = ".anchor/test-ledger-log.txt";

    if Path::new(test_ledger_filename).exists() {
        std::fs::remove_dir_all(test_ledger_filename)?;
    }
    if Path::new(test_ledger_log_filename).exists() {
        std::fs::remove_file(test_ledger_log_filename)?;
    }

    // Start a validator for testing.
    let test_validator_stdout = File::create(test_ledger_log_filename)?;
    let test_validator_stderr = test_validator_stdout.try_clone()?;
    let validator_handle = std::process::Command::new("solana-test-validator")
        .arg("--ledger")
        .arg(test_ledger_filename)
        .stdout(Stdio::from(test_validator_stdout))
        .stderr(Stdio::from(test_validator_stderr))
        .spawn()
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;

    // Wait for the validator to be ready.
    let client = RpcClient::new("http://localhost:8899".to_string());
    let mut count = 0;
    let ms_wait = 5000;
    while count < ms_wait {
        let r = client.get_recent_blockhash();
        if r.is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
        count += 1;
    }
    if count == 5000 {
        println!("Unable to start test validator.");
        std::process::exit(1);
    }

    Ok(validator_handle)
}

// TODO: Testing and deploys should use separate sections of metadata.
//       Similarly, each network should have separate metadata.
fn deploy(url: Option<String>, keypair: Option<String>) -> Result<()> {
    // Build all programs.
    set_workspace_dir_or_exit();
    build(None)?;
    set_workspace_dir_or_exit();

    // Deploy all programs.
    let cfg = Config::discover()?.expect("Inside a workspace").0;
    let url = url.unwrap_or(cfg.cluster.url().to_string());
    let keypair = keypair.unwrap_or(cfg.wallet.to_string());
    let deployment = deploy_ws(&url, &keypair)?;

    // Add metadata to all IDLs.
    for (program, address) in deployment {
        // Add metadata to the IDL.
        let mut idl = program.idl;
        idl.metadata = Some(serde_json::to_value(IdlTestMetadata {
            address: address.to_string(),
        })?);

        // Persist it.
        let idl_out = PathBuf::from("target/idl")
            .join(&idl.name)
            .with_extension("json");
        write_idl(&idl, Some(&idl_out))?;

        println!("Deployed {} at {}", idl.name, address.to_string());
    }

    run_hosted_deploy(&url, &keypair)?;

    Ok(())
}

fn run_hosted_deploy(url: &str, keypair: &str) -> Result<()> {
    println!("Running deploy script");

    let cur_dir = std::env::current_dir()?;
    let module_path = format!("{}/migrations/deploy.js", cur_dir.display());
    let deploy_script_host_str = template::deploy_script_host(url, &module_path);
    std::env::set_current_dir(".anchor")?;

    std::fs::write("deploy.js", deploy_script_host_str)?;
    if let Err(e) = std::process::Command::new("node")
        .arg("deploy.js")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
    {
        std::process::exit(1);
    }
    Ok(())
}

fn deploy_ws(url: &str, keypair: &str) -> Result<Vec<(Program, Pubkey)>> {
    let mut programs = vec![];
    println!("Deploying workspace to {}...", url);
    for program in read_all_programs()? {
        let binary_path = format!("target/deploy/{}.so", program.lib_name);
        println!("Deploying {}...", binary_path);
        let exit = std::process::Command::new("solana")
            .arg("deploy")
            .arg(&binary_path)
            .arg("--url")
            .arg(url)
            .arg("--keypair")
            .arg(keypair)
            .output()
            .expect("Must deploy");
        if !exit.status.success() {
            println!("There was a problem deploying: {:?}.", exit);
            std::process::exit(exit.status.code().unwrap_or(1));
        }
        let stdout: DeployStdout = serde_json::from_str(std::str::from_utf8(&exit.stdout)?)?;
        programs.push((program, stdout.program_id.parse()?));
    }
    println!("Deploy success!");
    Ok(programs)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeployStdout {
    program_id: String,
}

fn set_workspace_dir_or_exit() {
    let d = match Config::discover() {
        Err(_) => {
            println!("Not in anchor workspace.");
            std::process::exit(1);
        }
        Ok(d) => d,
    };
    match d {
        None => {
            println!("Not in anchor workspace.");
            std::process::exit(1);
        }
        Some((_cfg, cfg_path, _inside_cargo)) => {
            match cfg_path.parent() {
                None => {
                    println!("Unable to make new program");
                }
                Some(parent) => match std::env::set_current_dir(&parent) {
                    Err(_) => {
                        println!("Not in anchor workspace.");
                        std::process::exit(1);
                    }
                    Ok(_) => {}
                },
            };
        }
    }
}
