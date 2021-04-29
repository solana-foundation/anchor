//! CLI for workspace management of anchor programs.

use crate::config::{read_all_programs, Config, Program};
use anchor_lang::idl::{IdlAccount, IdlInstruction};
use anchor_lang::{AccountDeserialize, AnchorDeserialize, AnchorSerialize};
use anchor_syn::idl::Idl;
use anyhow::{anyhow, Context, Result};
use clap::Clap;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_sdk::account_utils::StateMut;
use solana_sdk::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signer;
use solana_sdk::sysvar;
use solana_sdk::transaction::Transaction;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::string::ToString;

mod config;
mod template;

// Version of the docker image.
const VERSION: &str = env!("CARGO_PKG_VERSION");
const DOCKER_BUILDER_VERSION: &str = VERSION;

#[derive(Debug, Clap)]
#[clap(version = VERSION)]
pub struct Opts {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clap)]
pub enum Command {
    /// Initializes a workspace.
    Init {
        name: String,
        #[clap(short, long)]
        typescript: bool,
    },
    /// Builds the workspace.
    Build {
        /// Output directory for the IDL.
        #[clap(short, long)]
        idl: Option<String>,
        /// True if the build artifact needs to be deterministic and verifiable.
        #[clap(short, long)]
        verifiable: bool,
    },
    /// Verifies the on-chain bytecode matches the locally compiled artifact.
    /// Run this command inside a program subdirectory, i.e., in the dir
    /// containing the program's Cargo.toml.
    Verify {
        /// The deployed program to compare against.
        program_id: Pubkey,
    },
    /// Runs integration tests against a localnetwork.
    Test {
        /// Use this flag if you want to run tests against previously deployed
        /// programs.
        #[clap(long)]
        skip_deploy: bool,
        /// Flag to skip starting a local validator, if the configured cluster
        /// url is a localnet.
        #[clap(long)]
        skip_local_validator: bool,
        file: Option<String>,
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
        /// True if the build should be verifiable. If deploying to mainnet,
        /// this should almost always be set.
        #[clap(short, long)]
        verifiable: bool,
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
    #[cfg(feature = "dev")]
    /// Runs an airdrop loop, continuously funding the configured wallet.
    Airdrop {
        #[clap(short, long)]
        url: Option<String>,
    },
    /// Cluster commands.
    Cluster {
        #[clap(subcommand)]
        subcmd: ClusterCommand,
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
    /// Writes an IDL into a buffer account. This can be used with SetBuffer
    /// to perform an upgrade.
    WriteBuffer {
        program_id: Pubkey,
        #[clap(short, long)]
        filepath: String,
    },
    /// Sets a new IDL buffer for the program.
    SetBuffer {
        program_id: Pubkey,
        /// Address of the buffer account to set as the idl on the program.
        #[clap(short, long)]
        buffer: Pubkey,
    },
    /// Upgrades the IDL to the new file. An alias for first writing and then
    /// then setting the idl buffer account.
    Upgrade {
        program_id: Pubkey,
        #[clap(short, long)]
        filepath: String,
    },
    /// Sets a new authority on the IDL account.
    SetAuthority {
        /// The IDL account buffer to set the authority of. If none is given,
        /// then the canonical IDL account is used.
        address: Option<Pubkey>,
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
    /// Fetches an IDL for the given address from a cluster.
    /// The address can be a program, IDL account, or IDL buffer.
    Fetch {
        address: Pubkey,
        /// Output file for the idl (stdout if not specified).
        #[clap(short, long)]
        out: Option<String>,
    },
}

#[derive(Debug, Clap)]
pub enum ClusterCommand {
    /// Prints common cluster urls.
    List,
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    match opts.command {
        Command::Init { name, typescript } => init(name, typescript),
        Command::New { name } => new(name),
        Command::Build { idl, verifiable } => build(idl, verifiable),
        Command::Verify { program_id } => verify(program_id),
        Command::Deploy { url, keypair } => deploy(url, keypair),
        Command::Upgrade {
            program_id,
            program_filepath,
        } => upgrade(program_id, program_filepath),
        Command::Idl { subcmd } => idl(subcmd),
        Command::Migrate { url } => migrate(url),
        Command::Launch {
            url,
            keypair,
            verifiable,
        } => launch(url, keypair, verifiable),
        Command::Test {
            skip_deploy,
            skip_local_validator,
            file,
        } => test(skip_deploy, skip_local_validator, file),
        #[cfg(feature = "dev")]
        Command::Airdrop { url } => airdrop(url),
        Command::Cluster { subcmd } => cluster(subcmd),
    }
}

fn init(name: String, typescript: bool) -> Result<()> {
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

    // Initialize .gitignore file
    let mut virt_manifest = File::create(".gitignore")?;
    virt_manifest.write_all(template::git_ignore().as_bytes())?;

    // Build the program.
    fs::create_dir("programs")?;

    new_program(&name)?;

    // Build the test suite.
    fs::create_dir("tests")?;
    // Build the migrations directory.
    fs::create_dir("migrations")?;

    if typescript {
        // Build typescript config
        let mut ts_config = File::create("tsconfig.json")?;
        ts_config.write_all(template::ts_config().as_bytes())?;

        let mut deploy = File::create("migrations/deploy.ts")?;
        deploy.write_all(&template::ts_deploy_script().as_bytes())?;

        let mut mocha = File::create(&format!("tests/{}.spec.ts", name))?;
        mocha.write_all(template::ts_mocha(&name).as_bytes())?;
    } else {
        let mut mocha = File::create(&format!("tests/{}.js", name))?;
        mocha.write_all(template::mocha(&name).as_bytes())?;

        let mut deploy = File::create("migrations/deploy.js")?;
        deploy.write_all(&template::deploy_script().as_bytes())?;
    }

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

fn build(idl: Option<String>, verifiable: bool) -> Result<()> {
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
        None => build_all(&cfg, path, idl_out, verifiable)?,
        Some(ct) => build_cwd(path.as_path(), ct, idl_out, verifiable)?,
    };

    set_workspace_dir_or_exit();

    Ok(())
}

fn build_all(
    _cfg: &Config,
    cfg_path: PathBuf,
    idl_out: Option<PathBuf>,
    verifiable: bool,
) -> Result<()> {
    let cur_dir = std::env::current_dir()?;
    let r = match cfg_path.parent() {
        None => Err(anyhow!("Invalid Anchor.toml at {}", cfg_path.display())),
        Some(parent) => {
            let files = fs::read_dir(parent.join("programs"))?;
            for f in files {
                let p = f?.path();
                build_cwd(
                    cfg_path.as_path(),
                    p.join("Cargo.toml"),
                    idl_out.clone(),
                    verifiable,
                )?;
            }
            Ok(())
        }
    };
    std::env::set_current_dir(cur_dir)?;
    r
}

// Runs the build command outside of a workspace.
fn build_cwd(
    cfg_path: &Path,
    cargo_toml: PathBuf,
    idl_out: Option<PathBuf>,
    verifiable: bool,
) -> Result<()> {
    match cargo_toml.parent() {
        None => return Err(anyhow!("Unable to find parent")),
        Some(p) => std::env::set_current_dir(&p)?,
    };
    match verifiable {
        false => _build_cwd(idl_out),
        true => build_cwd_verifiable(cfg_path.parent().unwrap()),
    }
}

// Builds an anchor program in a docker image and copies the build artifacts
// into the `target/` directory.
fn build_cwd_verifiable(workspace_dir: &Path) -> Result<()> {
    // Docker vars.
    let container_name = "anchor-program";
    let image_name = format!("projectserum/build:v{}", DOCKER_BUILDER_VERSION);
    let volume_mount = format!(
        "{}:/workdir",
        workspace_dir.canonicalize()?.display().to_string()
    );

    // Create output dirs.
    fs::create_dir_all(workspace_dir.join("target/deploy"))?;
    fs::create_dir_all(workspace_dir.join("target/idl"))?;

    // Build the program in docker.
    let exit = std::process::Command::new("docker")
        .args(&[
            "run",
            "--name",
            &container_name,
            "-v",
            &volume_mount,
            &image_name,
            "anchor",
            "build",
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;
    if !exit.status.success() {
        println!("Error building program");
        return Ok(());
    }

    let idl = extract_idl("src/lib.rs")?;

    // Copy the binary out of the docker image.
    let out_file = format!("../../target/deploy/{}.so", idl.name);
    let bin_artifact = format!("{}:/workdir/target/deploy/{}.so", container_name, idl.name);
    let exit = std::process::Command::new("docker")
        .args(&["cp", &bin_artifact, &out_file])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;
    if !exit.status.success() {
        return Ok(());
    }

    // Copy the idl out of the docker image.
    let out_file = format!("../../target/idl/{}.json", idl.name);
    let idl_artifact = format!("{}:/workdir/target/idl/{}.json", container_name, idl.name);
    let exit = std::process::Command::new("docker")
        .args(&["cp", &idl_artifact, &out_file])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;
    if !exit.status.success() {
        return Ok(());
    }

    // Remove the docker image.
    let exit = std::process::Command::new("docker")
        .args(&["rm", &container_name])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;
    if !exit.status.success() {
        std::process::exit(exit.status.code().unwrap_or(1));
    }

    Ok(())
}

fn _build_cwd(idl_out: Option<PathBuf>) -> Result<()> {
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

fn verify(program_id: Pubkey) -> Result<()> {
    let (cfg, _path, cargo) = Config::discover()?.expect("Not in workspace.");
    let cargo = cargo.ok_or(anyhow!("Must be inside program subdirectory."))?;
    let program_dir = cargo.parent().unwrap();

    // Build the program we want to verify.
    let cur_dir = std::env::current_dir()?;
    build(None, true)?;
    std::env::set_current_dir(&cur_dir)?;

    let local_idl = extract_idl("src/lib.rs")?;

    // Verify binary.
    let bin_path = program_dir
        .join("../../target/deploy/")
        .join(format!("{}.so", local_idl.name));
    let is_buffer = verify_bin(program_id, &bin_path, cfg.cluster.url())?;

    // Verify IDL (only if it's not a buffer account).
    if !is_buffer {
        std::env::set_current_dir(program_dir)?;
        let deployed_idl = fetch_idl(program_id)?;
        if local_idl != deployed_idl {
            println!("Error: IDLs don't match");
            std::process::exit(1);
        }
    }

    println!("{} is verified.", program_id);

    Ok(())
}

fn verify_bin(program_id: Pubkey, bin_path: &Path, cluster: &str) -> Result<bool> {
    let client = RpcClient::new(cluster.to_string());

    // Get the deployed build artifacts.
    let (deployed_bin, is_buffer) = {
        let account = client
            .get_account_with_commitment(&program_id, CommitmentConfig::default())?
            .value
            .map_or(Err(anyhow!("Account not found")), Ok)?;
        match account.state()? {
            UpgradeableLoaderState::Program {
                programdata_address,
            } => (
                client
                    .get_account_with_commitment(&programdata_address, CommitmentConfig::default())?
                    .value
                    .map_or(Err(anyhow!("Account not found")), Ok)?
                    .data[UpgradeableLoaderState::programdata_data_offset().unwrap_or(0)..]
                    .to_vec(),
                false,
            ),
            UpgradeableLoaderState::Buffer { .. } => {
                let offset = UpgradeableLoaderState::buffer_data_offset().unwrap_or(0);
                (account.data[offset..].to_vec(), true)
            }
            _ => return Err(anyhow!("Invalid program id")),
        }
    };
    let mut local_bin = {
        let mut f = File::open(bin_path)?;
        let mut contents = vec![];
        f.read_to_end(&mut contents)?;
        contents
    };

    // The deployed program probably has zero bytes appended. The default is
    // 2x the binary size in case of an upgrade.
    if local_bin.len() < deployed_bin.len() {
        local_bin.append(&mut vec![0; deployed_bin.len() - local_bin.len()]);
    }

    // Finally, check the bytes.
    if local_bin != deployed_bin {
        println!("Error: Binaries don't match");
        std::process::exit(1);
    }

    Ok(is_buffer)
}

// Fetches an IDL for the given program_id.
fn fetch_idl(idl_addr: Pubkey) -> Result<Idl> {
    let cfg = Config::discover()?.expect("Inside a workspace").0;
    let client = RpcClient::new(cfg.cluster.url().to_string());

    let mut account = client
        .get_account_with_commitment(&idl_addr, CommitmentConfig::processed())?
        .value
        .map_or(Err(anyhow!("Account not found")), Ok)?;

    if account.executable {
        let idl_addr = IdlAccount::address(&idl_addr);
        account = client
            .get_account_with_commitment(&idl_addr, CommitmentConfig::processed())?
            .value
            .map_or(Err(anyhow!("Account not found")), Ok)?;
    }

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
        IdlCommand::WriteBuffer {
            program_id,
            filepath,
        } => idl_write_buffer(program_id, filepath).map(|_| ()),
        IdlCommand::SetBuffer { program_id, buffer } => idl_set_buffer(program_id, buffer),
        IdlCommand::Upgrade {
            program_id,
            filepath,
        } => idl_upgrade(program_id, filepath),
        IdlCommand::SetAuthority {
            program_id,
            address,
            new_authority,
        } => idl_set_authority(program_id, address, new_authority),
        IdlCommand::EraseAuthority { program_id } => idl_erase_authority(program_id),
        IdlCommand::Authority { program_id } => idl_authority(program_id),
        IdlCommand::Parse { file, out } => idl_parse(file, out),
        IdlCommand::Fetch { address, out } => idl_fetch(address, out),
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

fn idl_write_buffer(program_id: Pubkey, idl_filepath: String) -> Result<Pubkey> {
    with_workspace(|cfg, _path, _cargo| {
        let keypair = cfg.wallet.to_string();

        let bytes = std::fs::read(idl_filepath)?;
        let idl: Idl = serde_json::from_reader(&*bytes)?;

        let idl_buffer = create_idl_buffer(&cfg, &keypair, &program_id, &idl)?;
        idl_write(&cfg, &program_id, &idl, idl_buffer)?;

        println!("Idl buffer created: {:?}", idl_buffer);

        Ok(idl_buffer)
    })
}

fn idl_set_buffer(program_id: Pubkey, buffer: Pubkey) -> Result<()> {
    with_workspace(|cfg, _path, _cargo| {
        let keypair = solana_sdk::signature::read_keypair_file(&cfg.wallet.to_string())
            .map_err(|_| anyhow!("Unable to read keypair file"))?;
        let client = RpcClient::new(cfg.cluster.url().to_string());

        // Instruction to set the buffer onto the IdlAccount.
        let set_buffer_ix = {
            let accounts = vec![
                AccountMeta::new(buffer, false),
                AccountMeta::new(IdlAccount::address(&program_id), false),
                AccountMeta::new(keypair.pubkey(), true),
            ];
            let mut data = anchor_lang::idl::IDL_IX_TAG.to_le_bytes().to_vec();
            data.append(&mut IdlInstruction::SetBuffer.try_to_vec()?);
            Instruction {
                program_id,
                accounts,
                data,
            }
        };

        // Build the transaction.
        let (recent_hash, _fee_calc) = client.get_recent_blockhash()?;
        let tx = Transaction::new_signed_with_payer(
            &[set_buffer_ix],
            Some(&keypair.pubkey()),
            &[&keypair],
            recent_hash,
        );

        // Send the transaction.
        client.send_and_confirm_transaction_with_spinner_and_config(
            &tx,
            CommitmentConfig::confirmed(),
            RpcSendTransactionConfig {
                skip_preflight: true,
                ..RpcSendTransactionConfig::default()
            },
        )?;

        Ok(())
    })
}

fn idl_upgrade(program_id: Pubkey, idl_filepath: String) -> Result<()> {
    let buffer = idl_write_buffer(program_id, idl_filepath)?;
    idl_set_buffer(program_id, buffer)
}

fn idl_authority(program_id: Pubkey) -> Result<()> {
    with_workspace(|cfg, _path, _cargo| {
        let client = RpcClient::new(cfg.cluster.url().to_string());
        let idl_address = {
            let account = client
                .get_account_with_commitment(&program_id, CommitmentConfig::processed())?
                .value
                .map_or(Err(anyhow!("Account not found")), Ok)?;
            if account.executable {
                IdlAccount::address(&program_id)
            } else {
                program_id
            }
        };

        let account = client.get_account(&idl_address)?;
        let mut data: &[u8] = &account.data;
        let idl_account: IdlAccount = AccountDeserialize::try_deserialize(&mut data)?;

        println!("{:?}", idl_account.authority);

        Ok(())
    })
}

fn idl_set_authority(
    program_id: Pubkey,
    address: Option<Pubkey>,
    new_authority: Pubkey,
) -> Result<()> {
    with_workspace(|cfg, _path, _cargo| {
        // Misc.
        let idl_address = match address {
            None => IdlAccount::address(&program_id),
            Some(addr) => addr,
        };
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
    idl_set_authority(program_id, None, new_authority)?;

    Ok(())
}

// Write the idl to the account buffer, chopping up the IDL into pieces
// and sending multiple transactions in the event the IDL doesn't fit into
// a single transaction.
fn idl_write(cfg: &Config, program_id: &Pubkey, idl: &Idl, idl_address: Pubkey) -> Result<()> {
    // Remove the metadata before deploy.
    let mut idl = idl.clone();
    idl.metadata = None;

    // Misc.
    let keypair = solana_sdk::signature::read_keypair_file(&cfg.wallet.to_string())
        .map_err(|_| anyhow!("Unable to read keypair file"))?;
    let client = RpcClient::new(cfg.cluster.url().to_string());

    // Serialize and compress the idl.
    let idl_data = {
        let json_bytes = serde_json::to_vec(&idl)?;
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

fn idl_fetch(address: Pubkey, out: Option<String>) -> Result<()> {
    let idl = fetch_idl(address)?;
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
fn test(skip_deploy: bool, skip_local_validator: bool, file: Option<String>) -> Result<()> {
    with_workspace(|cfg, _path, _cargo| {
        // Bootup validator, if needed.
        let validator_handle = match cfg.cluster.url() {
            "http://127.0.0.1:8899" => {
                build(None, false)?;
                let flags = match skip_deploy {
                    true => None,
                    false => Some(genesis_flags(cfg)?),
                };
                match skip_local_validator {
                    true => None,
                    false => Some(start_test_validator(cfg, flags)?),
                }
            }
            _ => {
                if !skip_deploy {
                    build(None, false)?;
                    deploy(None, None)?;
                }
                None
            }
        };

        // Setup log reader.
        let log_streams = stream_logs(&cfg.cluster.url());

        // Run the tests.
        let test_result: Result<_> = {
            let ts_config_exist = Path::new("tsconfig.json").exists();
            let mut args = vec!["-t", "1000000"];
            if let Some(ref file) = file {
                args.push(file);
            } else if ts_config_exist {
                args.push("tests/**/*.spec.ts");
            } else {
                args.push("tests/");
            }
            let exit = match ts_config_exist {
                true => std::process::Command::new("ts-mocha")
                    .arg("-p")
                    .arg("./tsconfig.json")
                    .args(args)
                    .env("ANCHOR_PROVIDER_URL", cfg.cluster.url())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .output()
                    .map_err(anyhow::Error::from)
                    .with_context(|| "ts-mocha"),
                false => std::process::Command::new("mocha")
                    .args(args)
                    .env("ANCHOR_PROVIDER_URL", cfg.cluster.url())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .output()
                    .map_err(anyhow::Error::from)
                    .with_context(|| "mocha"),
            };

            exit
        };

        // Check all errors and shut down.
        if let Some(mut child) = validator_handle {
            if let Err(err) = child.kill() {
                println!("Failed to kill subprocess {}: {}", child.id(), err);
            }
        }
        for mut child in log_streams? {
            if let Err(err) = child.kill() {
                println!("Failed to kill subprocess {}: {}", child.id(), err);
            }
        }
        match test_result {
            Ok(exit) => {
                if !exit.status.success() {
                    std::process::exit(exit.status.code().unwrap());
                }
            }
            Err(err) => {
                println!("Failed to run test: {:#}", err)
            }
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

fn start_test_validator(cfg: &Config, flags: Option<Vec<String>>) -> Result<Child> {
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
        .arg("--mint")
        .arg(cfg.wallet_kp()?.pubkey().to_string())
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

fn launch(url: Option<String>, keypair: Option<String>, verifiable: bool) -> Result<()> {
    // Build and deploy.
    build(None, verifiable)?;
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
        if Path::new("migrations/deploy.js").exists() || Path::new("migrations/deploy.ts").exists()
        {
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
    let idl_data = serialize_idl(idl)?;

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

    // Write directly to the IDL account buffer.
    idl_write(cfg, program_id, idl, IdlAccount::address(program_id))?;

    Ok(idl_address)
}

fn create_idl_buffer(
    cfg: &Config,
    keypair_path: &str,
    program_id: &Pubkey,
    idl: &Idl,
) -> Result<Pubkey> {
    let keypair = solana_sdk::signature::read_keypair_file(keypair_path)
        .map_err(|_| anyhow!("Unable to read keypair file"))?;
    let client = RpcClient::new(cfg.cluster.url().to_string());

    let buffer = Keypair::generate(&mut OsRng);

    // Creates the new buffer account with the system program.
    let create_account_ix = {
        let space = 8 + 32 + 4 + serialize_idl(idl)?.len() as usize;
        let lamports = client.get_minimum_balance_for_rent_exemption(space)?;
        solana_sdk::system_instruction::create_account(
            &keypair.pubkey(),
            &buffer.pubkey(),
            lamports,
            space as u64,
            program_id,
        )
    };

    // Program instruction to create the buffer.
    let create_buffer_ix = {
        let accounts = vec![
            AccountMeta::new(buffer.pubkey(), false),
            AccountMeta::new_readonly(keypair.pubkey(), true),
            AccountMeta::new_readonly(sysvar::rent::ID, false),
        ];
        let mut data = anchor_lang::idl::IDL_IX_TAG.to_le_bytes().to_vec();
        data.append(&mut IdlInstruction::CreateBuffer.try_to_vec()?);
        Instruction {
            program_id: *program_id,
            accounts,
            data,
        }
    };

    // Build the transaction.
    let (recent_hash, _fee_calc) = client.get_recent_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &[create_account_ix, create_buffer_ix],
        Some(&keypair.pubkey()),
        &[&keypair, &buffer],
        recent_hash,
    );

    // Send the transaction.
    client.send_and_confirm_transaction_with_spinner_and_config(
        &tx,
        CommitmentConfig::confirmed(),
        RpcSendTransactionConfig {
            skip_preflight: true,
            ..RpcSendTransactionConfig::default()
        },
    )?;

    Ok(buffer.pubkey())
}

// Serialize and compress the idl.
fn serialize_idl(idl: &Idl) -> Result<Vec<u8>> {
    let json_bytes = serde_json::to_vec(idl)?;
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(&json_bytes)?;
    e.finish().map_err(Into::into)
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
        let module_path = cur_dir.join("migrations/deploy.js");

        let ts_config_exist = Path::new("tsconfig.json").exists();
        let ts_deploy_file_exists = Path::new("migrations/deploy.ts").exists();

        if ts_config_exist && ts_deploy_file_exists {
            let ts_module_path = cur_dir.join("migrations/deploy.ts");
            let exit = std::process::Command::new("tsc")
                .arg(&ts_module_path)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()?;
            if !exit.status.success() {
                std::process::exit(exit.status.code().unwrap());
            }
        };

        let deploy_script_host_str =
            template::deploy_script_host(&url, &module_path.display().to_string());

        if !Path::new(".anchor").exists() {
            fs::create_dir(".anchor")?;
        }
        std::env::set_current_dir(".anchor")?;

        std::fs::write("deploy.js", deploy_script_host_str)?;
        let exit = std::process::Command::new("node")
            .arg("deploy.js")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()?;

        if ts_config_exist && ts_deploy_file_exists {
            std::fs::remove_file(&module_path)
                .map_err(|_| anyhow!("Unable to remove file {}", module_path.display()))?;
        }

        if !exit.status.success() {
            println!("Deploy failed.");
            std::process::exit(exit.status.code().unwrap());
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

#[cfg(feature = "dev")]
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

fn cluster(_cmd: ClusterCommand) -> Result<()> {
    println!("Cluster Endpoints:\n");
    println!("* Mainnet - https://solana-api.projectserum.com");
    println!("* Mainnet - https://api.mainnet-beta.solana.com");
    println!("* Devnet  - https://devnet.solana.com");
    println!("* Testnet - https://testnet.solana.com");
    Ok(())
}
