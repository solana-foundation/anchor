use crate::config::{read_all_programs, Config, Program};
use anchor_lang::idl::IdlAccount;
use anchor_lang::{AccountDeserialize, AnchorDeserialize, AnchorSerialize};
use anchor_syn::idl::Idl;
use anyhow::{anyhow, Result};
use clap::Clap;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
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
    /// Builds the workspace.
    Build {
        /// Output directory for the IDL.
        #[clap(short, long)]
        idl: Option<String>,
    },
    /// Runs integration tests against a localnetwork.
    Test {
        /// Use this flag if you want to run tests against previously deployed
        /// programs.
        #[clap(short, long)]
        skip_deploy: bool,
    },
    /// Creates a new program.
    New { name: String },
    /// Commands for interacting with interface definitions.
    Idl {
        #[clap(subcommand)]
        subcmd: IdlCommand,
    },
    /// Deploys each program in the workspace.
    Deploy {
        #[clap(short, long)]
        url: Option<String>,
        #[clap(short, long)]
        keypair: Option<String>,
    },
    /// Runs the deploy migration script.
    Migrate {
        #[clap(short, long)]
        url: Option<String>,
    },
    /// Deploys, initializes an IDL, and migrates all in one command.
    Launch {
        #[clap(short, long)]
        url: Option<String>,
        #[clap(short, long)]
        keypair: Option<String>,
    },
    /// Upgrades a single program. The configured wallet must be the upgrade
    /// authority.
    Upgrade {
        /// The program to upgrade.
        #[clap(short, long)]
        program_id: Pubkey,
        /// Filepath to the new program binary.
        program_filepath: String,
    },
    /// Runs an airdrop loop, continuously funding the configured wallet.
    Airdrop {
        #[clap(short, long)]
        url: Option<String>,
    },
}

#[derive(Debug, Clap)]
pub enum IdlCommand {
    /// Initializes a program's IDL account. Can only be run once.
    Init {
        program_id: Pubkey,
        #[clap(short, long)]
        filepath: String,
    },
    /// Upgrades the IDL to the new file.
    Upgrade {
        program_id: Pubkey,
        #[clap(short, long)]
        filepath: String,
    },
    /// Sets a new authority on the IDL account.
    SetAuthority {
        /// Program to change the IDL authority.
        #[clap(short, long)]
        program_id: Pubkey,
        /// New authority of the IDL account.
        #[clap(short, long)]
        new_authority: Pubkey,
    },
    /// Command to remove the ability to modify the IDL account. This should
    /// likely be used in conjection with eliminating an "upgrade authority" on
    /// the program.
    EraseAuthority {
        #[clap(short, long)]
        program_id: Pubkey,
    },
    /// Outputs the authority for the IDL account.
    Authority {
        /// The program to view.
        program_id: Pubkey,
    },
    /// Parses an IDL from source.
    Parse {
        /// Path to the program's interface definition.
        #[clap(short, long)]
        file: String,
        /// Output file for the idl (stdout if not specified).
        #[clap(short, long)]
        out: Option<String>,
    },
    /// Fetches an IDL for the given program from a cluster.
    Fetch {
        program_id: Pubkey,
        /// Output file for the idl (stdout if not specified).
        #[clap(short, long)]
        out: Option<String>,
    },
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    match opts.command {
        Command::Init { name } => init(name),
        Command::New { name } => new(name),
        Command::Build { idl } => build(idl),
        Command::Deploy { url, keypair } => deploy(url, keypair),
        Command::Upgrade {
            program_id,
            program_filepath,
        } => upgrade(program_id, program_filepath),
        Command::Idl { subcmd } => idl(subcmd),
        Command::Migrate { url } => migrate(url),
        Command::Launch { url, keypair } => launch(url, keypair),
        Command::Test { skip_deploy } => test(skip_deploy),
        Command::Airdrop { url } => airdrop(url),
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
    with_workspace(|_cfg, path, _cargo| {
        match path.parent() {
            None => {
                println!("Unable to make new program");
            }
            Some(parent) => {
                std::env::set_current_dir(&parent)?;
                new_program(&name)?;
                println!("Created new program.");
            }
        };
        Ok(())
    })
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
    let (cfg, path, cargo) = Config::discover()?.expect("Not in workspace.");
    let idl_out = match idl {
        Some(idl) => Some(PathBuf::from(idl)),
        None => {
            let cfg_parent = match path.parent() {
                None => return Err(anyhow!("Invalid Anchor.toml")),
                Some(parent) => parent,
            };
            fs::create_dir_all(cfg_parent.join("target/idl"))?;
            Some(cfg_parent.join("target/idl"))
        }
    };
    match cargo {
        None => build_all(&cfg, path, idl_out)?,
        Some(ct) => build_cwd(ct, idl_out)?,
    };

    set_workspace_dir_or_exit();

    Ok(())
}

fn build_all(_cfg: &Config, cfg_path: PathBuf, idl_out: Option<PathBuf>) -> Result<()> {
    match cfg_path.parent() {
        None => Err(anyhow!("Invalid Anchor.toml at {}", cfg_path.display())),
        Some(parent) => {
            let files = fs::read_dir(parent.join("programs"))?;
            for f in files {
                let p = f?.path();
                build_cwd(p.join("Cargo.toml"), idl_out.clone())?;
            }
            Ok(())
        }
    }
}

// Runs the build command outside of a workspace.
fn build_cwd(cargo_toml: PathBuf, idl_out: Option<PathBuf>) -> Result<()> {
    match cargo_toml.parent() {
        None => return Err(anyhow!("Unable to find parent")),
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

    write_idl(&idl, OutFile::File(out))
}

// Fetches an IDL for the given program_id.
fn fetch_idl(program_id: Pubkey) -> Result<Idl> {
    let cfg = Config::discover()?.expect("Inside a workspace").0;
    let client = RpcClient::new(cfg.cluster.url().to_string());

    let idl_addr = IdlAccount::address(&program_id);

    let account = client
        .get_account_with_commitment(&idl_addr, CommitmentConfig::processed())?
        .value
        .map_or(Err(anyhow!("Account not found")), Ok)?;

    // Cut off account discriminator.
    let mut d: &[u8] = &account.data[8..];
    let idl_account: IdlAccount = AnchorDeserialize::deserialize(&mut d)?;

    let mut z = ZlibDecoder::new(&idl_account.data[..]);
    let mut s = Vec::new();
    z.read_to_end(&mut s)?;
    serde_json::from_slice(&s[..]).map_err(Into::into)
}

fn extract_idl(file: &str) -> Result<Idl> {
    let file = shellexpand::tilde(file);
    anchor_syn::parser::file::parse(&*file)
}

fn idl(subcmd: IdlCommand) -> Result<()> {
    match subcmd {
        IdlCommand::Init {
            program_id,
            filepath,
        } => idl_init(program_id, filepath),
        IdlCommand::Upgrade {
            program_id,
            filepath,
        } => idl_upgrade(program_id, filepath),
        IdlCommand::SetAuthority {
            program_id,
            new_authority,
        } => idl_set_authority(program_id, new_authority),
        IdlCommand::EraseAuthority { program_id } => idl_erase_authority(program_id),
        IdlCommand::Authority { program_id } => idl_authority(program_id),
        IdlCommand::Parse { file, out } => idl_parse(file, out),
        IdlCommand::Fetch { program_id, out } => idl_fetch(program_id, out),
    }
}

fn idl_init(program_id: Pubkey, idl_filepath: String) -> Result<()> {
    with_workspace(|cfg, _path, _cargo| {
        let keypair = cfg.wallet.to_string();

        let bytes = std::fs::read(idl_filepath)?;
        let idl: Idl = serde_json::from_reader(&*bytes)?;

        let idl_address = create_idl_account(&cfg, &keypair, &program_id, &idl)?;

        println!("Idl account created: {:?}", idl_address);
        Ok(())
    })
}

fn idl_upgrade(program_id: Pubkey, idl_filepath: String) -> Result<()> {
    with_workspace(|cfg, _path, _cargo| {
        let bytes = std::fs::read(idl_filepath)?;
        let idl: Idl = serde_json::from_reader(&*bytes)?;

        idl_clear(cfg, &program_id)?;
        idl_write(cfg, &program_id, &idl)?;

        Ok(())
    })
}

fn idl_authority(program_id: Pubkey) -> Result<()> {
    with_workspace(|cfg, _path, _cargo| {
        let client = RpcClient::new(cfg.cluster.url().to_string());
        let idl_address = IdlAccount::address(&program_id);

        let account = client.get_account(&idl_address)?;
        let mut data: &[u8] = &account.data;
        let idl_account: IdlAccount = AccountDeserialize::try_deserialize(&mut data)?;

        println!("{:?}", idl_account.authority);

        Ok(())
    })
}

fn idl_set_authority(program_id: Pubkey, new_authority: Pubkey) -> Result<()> {
    with_workspace(|cfg, _path, _cargo| {
        // Misc.
        let idl_address = IdlAccount::address(&program_id);
        let keypair = solana_sdk::signature::read_keypair_file(&cfg.wallet.to_string())
            .map_err(|_| anyhow!("Unable to read keypair file"))?;
        let client = RpcClient::new(cfg.cluster.url().to_string());

        // Instruction data.
        let data =
            serialize_idl_ix(anchor_lang::idl::IdlInstruction::SetAuthority { new_authority })?;

        // Instruction accounts.
        let accounts = vec![
            AccountMeta::new(idl_address, false),
            AccountMeta::new_readonly(keypair.pubkey(), true),
        ];

        // Instruction.
        let ix = Instruction {
            program_id,
            accounts,
            data,
        };
        // Send transaction.
        let (recent_hash, _fee_calc) = client.get_recent_blockhash()?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&keypair.pubkey()),
            &[&keypair],
            recent_hash,
        );
        client.send_and_confirm_transaction_with_spinner_and_config(
            &tx,
            CommitmentConfig::confirmed(),
            RpcSendTransactionConfig {
                skip_preflight: true,
                ..RpcSendTransactionConfig::default()
            },
        )?;

        println!("Authority update complete.");

        Ok(())
    })
}

fn idl_erase_authority(program_id: Pubkey) -> Result<()> {
    println!("Are you sure you want to erase the IDL authority: [y/n]");

    let stdin = std::io::stdin();
    let mut stdin_lines = stdin.lock().lines();
    let input = stdin_lines.next().unwrap().unwrap();
    if input != "y" {
        println!("Not erasing.");
        return Ok(());
    }

    // Program will treat the zero authority as erased.
    let new_authority = Pubkey::new_from_array([0u8; 32]);
    idl_set_authority(program_id, new_authority)?;

    Ok(())
}

// Clears out *all* IDL data. The authority for the IDL must be the configured
// wallet.
fn idl_clear(cfg: &Config, program_id: &Pubkey) -> Result<()> {
    let idl_address = IdlAccount::address(program_id);
    let keypair = solana_sdk::signature::read_keypair_file(&cfg.wallet.to_string())
        .map_err(|_| anyhow!("Unable to read keypair file"))?;
    let client = RpcClient::new(cfg.cluster.url().to_string());

    let data = serialize_idl_ix(anchor_lang::idl::IdlInstruction::Clear)?;
    let accounts = vec![
        AccountMeta::new(idl_address, false),
        AccountMeta::new_readonly(keypair.pubkey(), true),
    ];
    let ix = Instruction {
        program_id: *program_id,
        accounts,
        data,
    };
    let (recent_hash, _fee_calc) = client.get_recent_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&keypair.pubkey()),
        &[&keypair],
        recent_hash,
    );
    client.send_and_confirm_transaction_with_spinner_and_config(
        &tx,
        CommitmentConfig::confirmed(),
        RpcSendTransactionConfig {
            skip_preflight: true,
            ..RpcSendTransactionConfig::default()
        },
    )?;

    Ok(())
}

// Write the idl to the account buffer, chopping up the IDL into pieces
// and sending multiple transactions in the event the IDL doesn't fit into
// a single transaction.
fn idl_write(cfg: &Config, program_id: &Pubkey, idl: &Idl) -> Result<()> {
    // Misc.
    let idl_address = IdlAccount::address(program_id);
    let keypair = solana_sdk::signature::read_keypair_file(&cfg.wallet.to_string())
        .map_err(|_| anyhow!("Unable to read keypair file"))?;
    let client = RpcClient::new(cfg.cluster.url().to_string());

    // Serialize and compress the idl.
    let idl_data = {
        let json_bytes = serde_json::to_vec(idl)?;
        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(&json_bytes)?;
        e.finish()?
    };

    const MAX_WRITE_SIZE: usize = 1000;
    let mut offset = 0;
    while offset < idl_data.len() {
        // Instruction data.
        let data = {
            let start = offset;
            let end = std::cmp::min(offset + MAX_WRITE_SIZE, idl_data.len());
            serialize_idl_ix(anchor_lang::idl::IdlInstruction::Write {
                data: idl_data[start..end].to_vec(),
            })?
        };
        // Instruction accounts.
        let accounts = vec![
            AccountMeta::new(idl_address, false),
            AccountMeta::new_readonly(keypair.pubkey(), true),
        ];
        // Instruction.
        let ix = Instruction {
            program_id: *program_id,
            accounts,
            data,
        };
        // Send transaction.
        let (recent_hash, _fee_calc) = client.get_recent_blockhash()?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&keypair.pubkey()),
            &[&keypair],
            recent_hash,
        );
        client.send_and_confirm_transaction_with_spinner_and_config(
            &tx,
            CommitmentConfig::confirmed(),
            RpcSendTransactionConfig {
                skip_preflight: true,
                ..RpcSendTransactionConfig::default()
            },
        )?;
        offset += MAX_WRITE_SIZE;
    }
    Ok(())
}

fn idl_parse(file: String, out: Option<String>) -> Result<()> {
    let idl = extract_idl(&file)?;
    let out = match out {
        None => OutFile::Stdout,
        Some(out) => OutFile::File(PathBuf::from(out)),
    };
    write_idl(&idl, out)
}

fn idl_fetch(program_id: Pubkey, out: Option<String>) -> Result<()> {
    let idl = fetch_idl(program_id)?;
    let out = match out {
        None => OutFile::Stdout,
        Some(out) => OutFile::File(PathBuf::from(out)),
    };
    write_idl(&idl, out)
}

fn write_idl(idl: &Idl, out: OutFile) -> Result<()> {
    let idl_json = serde_json::to_string_pretty(idl)?;
    match out {
        OutFile::Stdout => println!("{}", idl_json),
        OutFile::File(out) => std::fs::write(out, idl_json)?,
    };
    Ok(())
}

enum OutFile {
    Stdout,
    File(PathBuf),
}

// Builds, deploys, and tests all workspace programs in a single command.
fn test(skip_deploy: bool) -> Result<()> {
    with_workspace(|cfg, _path, _cargo| {
        // Bootup validator, if needed.
        let validator_handle = match cfg.cluster.url() {
            "http://127.0.0.1:8899" => {
                build(None)?;
                let flags = match skip_deploy {
                    true => None,
                    false => Some(genesis_flags(cfg)?),
                };
                Some(start_test_validator(flags)?)
            }
            _ => {
                if !skip_deploy {
                    deploy(None, None)?;
                }
                None
            }
        };

        let log_streams = stream_logs(&cfg.cluster.url())?;

        // Run the tests.
        let exit = std::process::Command::new("mocha")
            .arg("-t")
            .arg("1000000")
            .arg("tests/")
            .env("ANCHOR_PROVIDER_URL", cfg.cluster.url())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()?;

        if !exit.status.success() {
            if let Some(mut validator_handle) = validator_handle {
                validator_handle.kill()?;
            }
            std::process::exit(exit.status.code().unwrap());
        }
        if let Some(mut validator_handle) = validator_handle {
            validator_handle.kill()?;
        }

        for mut stream in log_streams {
            stream.kill()?;
        }

        Ok(())
    })
}

// Returns the solana-test-validator flags to embed the workspace programs
// in the genesis block. This allows us to run tests without every deploying.
fn genesis_flags(cfg: &Config) -> Result<Vec<String>> {
    let mut flags = Vec::new();
    for mut program in read_all_programs()? {
        let binary_path = program.binary_path().display().to_string();

        let kp = Keypair::generate(&mut OsRng);
        let address = kp.pubkey().to_string();

        flags.push("--bpf-program".to_string());
        flags.push(address.clone());
        flags.push(binary_path);

        // Add program address to the IDL.
        program.idl.metadata = Some(serde_json::to_value(IdlTestMetadata { address })?);

        // Persist it.
        let idl_out = PathBuf::from("target/idl")
            .join(&program.idl.name)
            .with_extension("json");
        write_idl(&program.idl, OutFile::File(idl_out))?;
    }
    if let Some(test) = cfg.test.as_ref() {
        for entry in &test.genesis {
            flags.push("--bpf-program".to_string());
            flags.push(entry.address.clone());
            flags.push(entry.program.clone());
        }
    }
    Ok(flags)
}

fn stream_logs(url: &str) -> Result<Vec<std::process::Child>> {
    let program_logs_dir = ".anchor/program-logs";
    if Path::new(program_logs_dir).exists() {
        std::fs::remove_dir_all(program_logs_dir)?;
    }
    fs::create_dir_all(program_logs_dir)?;
    let mut handles = vec![];
    for program in read_all_programs()? {
        let mut file = File::open(&format!("target/idl/{}.json", program.lib_name))?;
        let mut contents = vec![];
        file.read_to_end(&mut contents)?;
        let idl: Idl = serde_json::from_slice(&contents)?;
        let metadata = idl
            .metadata
            .ok_or_else(|| anyhow!("Program address not found."))?;
        let metadata: IdlTestMetadata = serde_json::from_value(metadata)?;

        let log_file = File::create(format!(
            "{}/{}.{}.log",
            program_logs_dir, metadata.address, program.idl.name
        ))?;
        let stdio = std::process::Stdio::from(log_file);
        let child = std::process::Command::new("solana")
            .arg("logs")
            .arg(metadata.address)
            .arg("--url")
            .arg(url)
            .stdout(stdio)
            .spawn()?;
        handles.push(child);
    }
    Ok(handles)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdlTestMetadata {
    address: String,
}

fn start_test_validator(flags: Option<Vec<String>>) -> Result<Child> {
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
        .args(flags.unwrap_or_default())
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
    _deploy(url, keypair).map(|_| ())
}

fn _deploy(url: Option<String>, keypair: Option<String>) -> Result<Vec<(Pubkey, Program)>> {
    with_workspace(|cfg, _path, _cargo| {
        build(None)?;

        // Fallback to config vars if not provided via CLI.
        let url = url.unwrap_or_else(|| cfg.cluster.url().to_string());
        let keypair = keypair.unwrap_or_else(|| cfg.wallet.to_string());

        // Deploy the programs.
        println!("Deploying workspace: {}", url);
        println!("Upgrade authority: {}", keypair);

        let mut programs = Vec::new();

        for mut program in read_all_programs()? {
            let binary_path = program.binary_path().display().to_string();

            println!("Deploying {}...", binary_path);

            // Write the program's keypair filepath. This forces a new deploy
            // address.
            let program_kp = Keypair::generate(&mut OsRng);
            let mut file = File::create(program.anchor_keypair_path())?;
            file.write_all(format!("{:?}", &program_kp.to_bytes()).as_bytes())?;

            // Send deploy transactions.
            let exit = std::process::Command::new("solana")
                .arg("program")
                .arg("deploy")
                .arg("--url")
                .arg(&url)
                .arg("--keypair")
                .arg(&keypair)
                .arg("--program-id")
                .arg(program.anchor_keypair_path().display().to_string())
                .arg(&binary_path)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()
                .expect("Must deploy");
            if !exit.status.success() {
                println!("There was a problem deploying: {:?}.", exit);
                std::process::exit(exit.status.code().unwrap_or(1));
            }

            // Add program address to the IDL.
            program.idl.metadata = Some(serde_json::to_value(IdlTestMetadata {
                address: program_kp.pubkey().to_string(),
            })?);

            // Persist it.
            let idl_out = PathBuf::from("target/idl")
                .join(&program.idl.name)
                .with_extension("json");
            write_idl(&program.idl, OutFile::File(idl_out))?;

            programs.push((program_kp.pubkey(), program))
        }

        println!("Deploy success");

        Ok(programs)
    })
}

fn upgrade(program_id: Pubkey, program_filepath: String) -> Result<()> {
    let path: PathBuf = program_filepath.parse().unwrap();
    let program_filepath = path.canonicalize()?.display().to_string();

    with_workspace(|cfg, _path, _cargo| {
        let exit = std::process::Command::new("solana")
            .arg("program")
            .arg("deploy")
            .arg("--url")
            .arg(cfg.cluster.url())
            .arg("--keypair")
            .arg(&cfg.wallet.to_string())
            .arg("--program-id")
            .arg(program_id.to_string())
            .arg(&program_filepath)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .expect("Must deploy");
        if !exit.status.success() {
            println!("There was a problem deploying: {:?}.", exit);
            std::process::exit(exit.status.code().unwrap_or(1));
        }
        Ok(())
    })
}

fn launch(url: Option<String>, keypair: Option<String>) -> Result<()> {
    // Build and deploy.
    let programs = _deploy(url.clone(), keypair.clone())?;

    with_workspace(|cfg, _path, _cargo| {
        let url = url.unwrap_or_else(|| cfg.cluster.url().to_string());
        let keypair = keypair.unwrap_or_else(|| cfg.wallet.to_string());

        // Add metadata to all IDLs.
        for (address, program) in programs {
            // Store the IDL on chain.
            let idl_address = create_idl_account(&cfg, &keypair, &address, &program.idl)?;
            println!("IDL account created: {}", idl_address.to_string());
        }

        // Run migration script.
        if Path::new("migrations/deploy.js").exists() {
            migrate(Some(url))?;
        }

        Ok(())
    })
}

// with_workspace ensures the current working directory is always the top level
// workspace directory, i.e., where the `Anchor.toml` file is located, before
// and after the closure invocation.
//
// The closure passed into this function must never change the working directory
// to be outside the workspace. Doing so will have undefined behavior.
fn with_workspace<R>(f: impl FnOnce(&Config, PathBuf, Option<PathBuf>) -> R) -> R {
    set_workspace_dir_or_exit();

    clear_program_keys().unwrap();

    let (cfg, cfg_path, cargo_toml) = Config::discover()
        .expect("Previously set the workspace dir")
        .expect("Anchor.toml must always exist");
    let r = f(&cfg, cfg_path, cargo_toml);

    set_workspace_dir_or_exit();
    clear_program_keys().unwrap();

    r
}

// The Solana CLI doesn't redeploy a program if this file exists.
// So remove it to make all commands explicit.
fn clear_program_keys() -> Result<()> {
    for program in read_all_programs()? {
        let anchor_keypair_path = program.anchor_keypair_path();
        if Path::exists(&anchor_keypair_path) {
            std::fs::remove_file(anchor_keypair_path).expect("Always remove");
        }
    }
    Ok(())
}

fn create_idl_account(
    cfg: &Config,
    keypair_path: &str,
    program_id: &Pubkey,
    idl: &Idl,
) -> Result<Pubkey> {
    // Misc.
    let idl_address = IdlAccount::address(program_id);
    let keypair = solana_sdk::signature::read_keypair_file(keypair_path)
        .map_err(|_| anyhow!("Unable to read keypair file"))?;
    let client = RpcClient::new(cfg.cluster.url().to_string());

    // Serialize and compress the idl.
    let idl_data = {
        let json_bytes = serde_json::to_vec(idl)?;
        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(&json_bytes)?;
        e.finish()?
    };

    // Run `Create instruction.
    {
        let data = serialize_idl_ix(anchor_lang::idl::IdlInstruction::Create {
            data_len: (idl_data.len() as u64) * 2, // Double for future growth.
        })?;
        let program_signer = Pubkey::find_program_address(&[], program_id).0;
        let accounts = vec![
            AccountMeta::new_readonly(keypair.pubkey(), true),
            AccountMeta::new(idl_address, false),
            AccountMeta::new_readonly(program_signer, false),
            AccountMeta::new_readonly(solana_program::system_program::ID, false),
            AccountMeta::new_readonly(*program_id, false),
            AccountMeta::new_readonly(solana_program::sysvar::rent::ID, false),
        ];
        let ix = Instruction {
            program_id: *program_id,
            accounts,
            data,
        };
        let (recent_hash, _fee_calc) = client.get_recent_blockhash()?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&keypair.pubkey()),
            &[&keypair],
            recent_hash,
        );
        client.send_and_confirm_transaction_with_spinner_and_config(
            &tx,
            CommitmentConfig::confirmed(),
            RpcSendTransactionConfig {
                skip_preflight: true,
                ..RpcSendTransactionConfig::default()
            },
        )?;
    }

    idl_write(cfg, program_id, idl)?;

    Ok(idl_address)
}

fn serialize_idl_ix(ix_inner: anchor_lang::idl::IdlInstruction) -> Result<Vec<u8>> {
    let mut data = anchor_lang::idl::IDL_IX_TAG.to_le_bytes().to_vec();
    data.append(&mut ix_inner.try_to_vec()?);
    Ok(data)
}

fn migrate(url: Option<String>) -> Result<()> {
    with_workspace(|cfg, _path, _cargo| {
        println!("Running migration deploy script");

        let url = url.unwrap_or_else(|| cfg.cluster.url().to_string());
        let cur_dir = std::env::current_dir()?;
        let module_path = format!("{}/migrations/deploy.js", cur_dir.display());
        let deploy_script_host_str = template::deploy_script_host(&url, &module_path);
        std::env::set_current_dir(".anchor")?;

        std::fs::write("deploy.js", deploy_script_host_str)?;
        if let Err(_e) = std::process::Command::new("node")
            .arg("deploy.js")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
        {
            std::process::exit(1);
        }

        println!("Deploy complete.");
        Ok(())
    })
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
                Some(parent) => {
                    if std::env::set_current_dir(&parent).is_err() {
                        println!("Not in anchor workspace.");
                        std::process::exit(1);
                    }
                }
            };
        }
    }
}

fn airdrop(url: Option<String>) -> Result<()> {
    let url = url.unwrap_or_else(|| "https://devnet.solana.com".to_string());
    loop {
        let exit = std::process::Command::new("solana")
            .arg("airdrop")
            .arg("10")
            .arg("--url")
            .arg(&url)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .expect("Must airdrop");
        if !exit.status.success() {
            println!("There was a problem airdropping: {:?}.", exit);
            std::process::exit(exit.status.code().unwrap_or(1));
        }
        std::thread::sleep(std::time::Duration::from_millis(10000));
    }
}
