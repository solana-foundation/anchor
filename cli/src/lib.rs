use crate::config::{
    get_default_ledger_path, BootstrapMode, BuildConfig, Config, ConfigOverride, HookType,
    Manifest, PackageManager, ProgramArch, ProgramDeployment, ProgramWorkspace, ScriptsConfig,
    TestValidator, WithPath, SHUTDOWN_WAIT, STARTUP_WAIT,
};
use anchor_client::Cluster;
use anchor_lang::prelude::UpgradeableLoaderState;
use anchor_lang::solana_program::bpf_loader_upgradeable;
use anchor_lang::AnchorDeserialize;
use anchor_lang_idl::convert::convert_idl;
use anchor_lang_idl::types::{Idl, IdlArrayLen, IdlDefinedFields, IdlType, IdlTypeDefTy};
use anyhow::{anyhow, bail, Context, Result};
use checks::{check_anchor_version, check_deps, check_idl_build_feature, check_overflow};
use clap::{CommandFactory, Parser};
use dirs::home_dir;
use heck::{ToKebabCase, ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use regex::{Regex, RegexBuilder};
use rust_template::{ProgramTemplate, TestTemplate};
use semver::{Version, VersionReq};
use serde_json::{json, Map, Value as JsonValue};
use solana_commitment_config::CommitmentConfig;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;
use solana_signer::{EncodableKey, Signer};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::string::ToString;
use std::sync::LazyLock;

mod checks;
pub mod config;
pub mod rust_template;

// Version of the docker image.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DOCKER_BUILDER_VERSION: &str = VERSION;

/// Default RPC port
pub const DEFAULT_RPC_PORT: u16 = 8899;

pub static AVM_HOME: LazyLock<PathBuf> = LazyLock::new(|| {
    if let Ok(avm_home) = std::env::var("AVM_HOME") {
        PathBuf::from(avm_home)
    } else {
        let mut user_home = dirs::home_dir().expect("Could not find home directory");
        user_home.push(".avm");
        user_home
    }
});

#[derive(Debug, Parser)]
#[clap(version = VERSION)]
pub struct Opts {
    #[clap(flatten)]
    pub cfg_override: ConfigOverride,
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Parser)]
pub enum Command {
    /// Initializes a workspace.
    Init {
        /// Workspace name
        name: String,
        /// Use JavaScript instead of TypeScript
        #[clap(short, long)]
        javascript: bool,
        /// Don't install JavaScript dependencies
        #[clap(long)]
        no_install: bool,
        /// Package Manager to use
        #[clap(value_enum, long, default_value = "yarn")]
        package_manager: PackageManager,
        /// Don't initialize git
        #[clap(long)]
        no_git: bool,
        /// Rust program template to use
        #[clap(value_enum, short, long, default_value = "multiple")]
        template: ProgramTemplate,
        /// Test template to use
        #[clap(value_enum, long, default_value = "mocha")]
        test_template: TestTemplate,
        /// Initialize even if there are files
        #[clap(long, action)]
        force: bool,
    },
    /// Builds the workspace.
    #[clap(name = "build", alias = "b")]
    Build {
        /// True if the build should not fail even if there are no "CHECK" comments
        #[clap(long)]
        skip_lint: bool,
        /// Skip checking for program ID mismatch between keypair and declare_id
        #[clap(long)]
        ignore_keys: bool,
        /// Do not build the IDL
        #[clap(long)]
        no_idl: bool,
        /// Output directory for the IDL.
        #[clap(short, long)]
        idl: Option<String>,
        /// Output directory for the TypeScript IDL.
        #[clap(short = 't', long)]
        idl_ts: Option<String>,
        /// True if the build artifact needs to be deterministic and verifiable.
        #[clap(short, long)]
        verifiable: bool,
        /// Name of the program to build
        #[clap(short, long)]
        program_name: Option<String>,
        /// Version of the Solana toolchain to use. For --verifiable builds
        /// only.
        #[clap(short, long)]
        solana_version: Option<String>,
        /// Docker image to use. For --verifiable builds only.
        #[clap(short, long)]
        docker_image: Option<String>,
        /// Bootstrap docker image from scratch, installing all requirements for
        /// verifiable builds. Only works for debian-based images.
        #[clap(value_enum, short, long, default_value = "none")]
        bootstrap: BootstrapMode,
        /// Environment variables to pass into the docker container
        #[clap(short, long, required = false)]
        env: Vec<String>,
        /// Arguments to pass to the underlying `cargo build-sbf` command
        #[clap(required = false, last = true)]
        cargo_args: Vec<String>,
        /// Suppress doc strings in IDL output
        #[clap(long)]
        no_docs: bool,
        /// Architecture to use when building the program
        #[clap(value_enum, long, default_value = "sbf")]
        arch: ProgramArch,
    },
    /// Expands macros (wrapper around cargo expand)
    ///
    /// Use it in a program folder to expand program
    ///
    /// Use it in a workspace but outside a program
    /// folder to expand the entire workspace
    Expand {
        /// Expand only this program
        #[clap(short, long)]
        program_name: Option<String>,
        /// Arguments to pass to the underlying `cargo expand` command
        #[clap(required = false, last = true)]
        cargo_args: Vec<String>,
    },
    /// Verifies the on-chain bytecode matches the locally compiled artifact.
    /// Run this command inside a program subdirectory, i.e., in the dir
    /// containing the program's Cargo.toml.
    Verify {
        /// The program ID to verify.
        program_id: Pubkey,
        /// The URL of the repository to verify against. Conflicts with `--current-dir`.
        #[clap(long, conflicts_with = "current_dir")]
        repo_url: Option<String>,
        /// The commit hash to verify against. Requires `--repo-url`.
        #[clap(long, requires = "repo_url")]
        commit_hash: Option<String>,
        /// Verify against the source code in the current directory. Conflicts with `--repo-url`.
        #[clap(long)]
        current_dir: bool,
        /// Name of the program to run the command on. Defaults to the package name.
        #[clap(long)]
        program_name: Option<String>,
        /// Any additional arguments to pass to `solana-verify`.
        #[clap(raw = true)]
        args: Vec<String>,
    },
    #[clap(name = "test", alias = "t")]
    /// Runs integration tests.
    Test {
        /// Build and test only this program
        #[clap(short, long)]
        program_name: Option<String>,
        /// Use this flag if you want to run tests against previously deployed
        /// programs.
        #[clap(long)]
        skip_deploy: bool,
        /// True if the build should not fail even if there are
        /// no "CHECK" comments where normally required
        #[clap(long)]
        skip_lint: bool,
        /// Flag to skip starting a local validator, if the configured cluster
        /// url is a localnet.
        #[clap(long)]
        skip_local_validator: bool,
        /// Flag to skip building the program in the workspace,
        /// use this to save time when running test and the program code is not altered.
        #[clap(long)]
        skip_build: bool,
        /// Do not build the IDL
        #[clap(long)]
        no_idl: bool,
        /// Architecture to use when building the program
        #[clap(value_enum, long, default_value = "sbf")]
        arch: ProgramArch,
        /// Flag to keep the local validator running after tests
        /// to be able to check the transactions.
        #[clap(long)]
        detach: bool,
        /// Run the test suites under the specified path
        #[clap(long)]
        run: Vec<String>,
        args: Vec<String>,
        /// Environment variables to pass into the docker container
        #[clap(short, long, required = false)]
        env: Vec<String>,
        /// Arguments to pass to the underlying `cargo build-sbf` command.
        #[clap(required = false, last = true)]
        cargo_args: Vec<String>,
    },
    /// Creates a new program.
    New {
        /// Program name
        name: String,
        /// Rust program template to use
        #[clap(value_enum, short, long, default_value = "multiple")]
        template: ProgramTemplate,
        /// Create new program even if there is already one
        #[clap(long, action)]
        force: bool,
    },
    /// Commands for interacting with interface definitions.
    Idl {
        #[clap(subcommand)]
        subcmd: IdlCommand,
    },
    /// Remove all artifacts from the generated directories except program keypairs.
    Clean,
    /// Deploys each program in the workspace.
    Deploy {
        /// Only deploy this program
        #[clap(short, long)]
        program_name: Option<String>,
        /// Keypair of the program (filepath) (requires program-name)
        #[clap(long, requires = "program_name")]
        program_keypair: Option<String>,
        /// If true, deploy from path target/verifiable
        #[clap(short, long)]
        verifiable: bool,
        /// Don't upload IDL during deployment (IDL is uploaded by default)
        #[clap(long)]
        no_idl: bool,
        /// Arguments to pass to the underlying `solana program deploy` command.
        #[clap(required = false, last = true)]
        solana_args: Vec<String>,
    },
    /// Runs the deploy migration script.
    Migrate,
    /// Deploys, initializes an IDL, and migrates all in one command.
    /// Upgrades a single program. The configured wallet must be the upgrade
    /// authority.
    Upgrade {
        /// The program to upgrade.
        #[clap(short, long)]
        program_id: Pubkey,
        /// Filepath to the new program binary.
        program_filepath: String,
        /// Max times to retry on failure.
        #[clap(long, default_value = "0")]
        max_retries: u32,
        /// Arguments to pass to the underlying `solana program deploy` command.
        #[clap(required = false, last = true)]
        solana_args: Vec<String>,
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
    /// Starts a node shell with an Anchor client setup according to the local
    /// config.
    Shell,
    /// Runs the script defined by the current workspace's Anchor.toml.
    Run {
        /// The name of the script to run.
        script: String,
        /// Argument to pass to the underlying script.
        #[clap(required = false, last = true)]
        script_args: Vec<String>,
    },
    /// Saves an api token from the registry locally.
    Login {
        /// API access token.
        token: String,
    },
    /// Program keypair commands.
    Keys {
        #[clap(subcommand)]
        subcmd: KeysCommand,
    },
    /// Localnet commands.
    Localnet {
        /// Flag to skip building the program in the workspace,
        /// use this to save time when running test and the program code is not altered.
        #[clap(long)]
        skip_build: bool,
        /// Use this flag if you want to run tests against previously deployed
        /// programs.
        #[clap(long)]
        skip_deploy: bool,
        /// True if the build should not fail even if there are
        /// no "CHECK" comments where normally required
        #[clap(long)]
        skip_lint: bool,
        /// Skip checking for program ID mismatch between keypair and declare_id
        #[clap(long)]
        ignore_keys: bool,
        /// Architecture to use when building the program
        #[clap(value_enum, long, default_value = "sbf")]
        arch: ProgramArch,
        /// Environment variables to pass into the docker container
        #[clap(short, long, required = false)]
        env: Vec<String>,
        /// Arguments to pass to the underlying `cargo build-sbf` command.
        #[clap(required = false, last = true)]
        cargo_args: Vec<String>,
    },
    /// Fetch and deserialize an account using the IDL provided.
    Account {
        /// Account struct to deserialize
        account_type: String,
        /// Address of the account to deserialize
        address: Pubkey,
        /// IDL to use (defaults to workspace IDL)
        #[clap(long)]
        idl: Option<String>,
    },
    /// Generates shell completions.
    Completions {
        #[clap(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Debug, Parser)]
pub enum KeysCommand {
    /// List all of the program keys.
    List,
    /// Sync program `declare_id!` pubkeys with the program's actual pubkey.
    Sync {
        /// Only sync the given program instead of all programs
        #[clap(short, long)]
        program_name: Option<String>,
    },
}

#[derive(Debug, Parser)]
pub enum IdlCommand {
    /// Initializes a program's IDL account. Can only be run once.
    Init {
        program_id: Pubkey,
        #[clap(short, long)]
        filepath: String,
        #[clap(long)]
        priority_fee: Option<u64>,
        /// Create non-canonical metadata account (third-party metadata)
        #[clap(long)]
        non_canonical: bool,
    },
    /// Upgrades the IDL to the new file. An alias for first writing and then
    /// then setting the idl buffer account.
    Upgrade {
        program_id: Pubkey,
        #[clap(short, long)]
        filepath: String,
        #[clap(long)]
        priority_fee: Option<u64>,
    },
    /// Generates the IDL for the program using the compilation method.
    #[clap(alias = "b")]
    Build {
        // Program name to build the IDL of(current dir's program if not specified)
        #[clap(short, long)]
        program_name: Option<String>,
        /// Output file for the IDL (stdout if not specified)
        #[clap(short, long)]
        out: Option<String>,
        /// Output file for the TypeScript IDL
        #[clap(short = 't', long)]
        out_ts: Option<String>,
        /// Suppress doc strings in output
        #[clap(long)]
        no_docs: bool,
        /// Do not check for safety comments
        #[clap(long)]
        skip_lint: bool,
        /// Arguments to pass to the underlying `cargo test` command
        #[clap(required = false, last = true)]
        cargo_args: Vec<String>,
    },
    /// Fetches an IDL for the given address from a cluster.
    /// The address can be a program, IDL account, or IDL buffer.
    Fetch {
        address: Pubkey,
        /// Output file for the IDL (stdout if not specified).
        #[clap(short, long)]
        out: Option<String>,
        /// Fetch non-canonical metadata account (third-party metadata)
        #[clap(long)]
        non_canonical: bool,
    },
    /// Convert legacy IDLs (pre Anchor 0.30) to the new IDL spec
    Convert {
        /// Path to the IDL file
        path: String,
        /// Output file for the IDL (stdout if not specified)
        #[clap(short, long)]
        out: Option<String>,
        /// Address to use (defaults to `metadata.address` value)
        #[clap(short, long)]
        program_id: Option<Pubkey>,
    },
    /// Generate TypeScript type for the IDL
    Type {
        /// Path to the IDL file
        path: String,
        /// Output file for the IDL (stdout if not specified)
        #[clap(short, long)]
        out: Option<String>,
    },
    /// Close a metadata account and recover rent
    Close {
        /// The program ID
        program_id: Pubkey,
        /// The seed used for the metadata account (default: "idl")
        #[clap(long, default_value = "idl")]
        seed: String,
        /// Priority fees in micro-lamports per compute unit
        #[clap(long)]
        priority_fee: Option<u64>,
    },
    /// Create a buffer account for metadata
    CreateBuffer {
        /// Path to the metadata file
        #[clap(short, long)]
        filepath: String,
        /// Priority fees in micro-lamports per compute unit
        #[clap(long)]
        priority_fee: Option<u64>,
    },
    /// Set a new authority on a buffer account
    SetBufferAuthority {
        /// The buffer account address
        buffer: Pubkey,
        /// The new authority
        #[clap(short, long)]
        new_authority: Pubkey,
        /// Priority fees in micro-lamports per compute unit
        #[clap(long)]
        priority_fee: Option<u64>,
    },
    /// Write metadata using a buffer account
    WriteBuffer {
        /// The program ID
        program_id: Pubkey,
        /// The buffer account address
        #[clap(short, long)]
        buffer: Pubkey,
        /// The seed to use for the metadata account (default: "idl")
        #[clap(long, default_value = "idl")]
        seed: String,
        /// Close the buffer after writing
        #[clap(long)]
        close_buffer: bool,
        /// Priority fees in micro-lamports per compute unit
        #[clap(long)]
        priority_fee: Option<u64>,
    },
}

#[derive(Debug, Parser)]
pub enum ClusterCommand {
    /// Prints common cluster urls.
    List,
}

fn get_keypair(path: &str) -> Result<Keypair> {
    solana_keypair::read_keypair_file(path)
        .map_err(|_| anyhow!("Unable to read keypair file ({path})"))
}

pub fn entry(opts: Opts) -> Result<()> {
    let restore_cbs = override_toolchain(&opts.cfg_override)?;
    let result = process_command(opts);
    restore_toolchain(restore_cbs)?;

    result
}

/// Functions to restore toolchain entries
type RestoreToolchainCallbacks = Vec<Box<dyn FnOnce() -> Result<()>>>;

/// Override the toolchain from `Anchor.toml`.
///
/// Returns the previous versions to restore back to.
fn override_toolchain(cfg_override: &ConfigOverride) -> Result<RestoreToolchainCallbacks> {
    let mut restore_cbs: RestoreToolchainCallbacks = vec![];

    let cfg = Config::discover(cfg_override)?;
    if let Some(cfg) = cfg {
        fn parse_version(text: &str) -> Option<String> {
            Some(
                Regex::new(r"(\d+\.\d+\.\S+)")
                    .unwrap()
                    .captures_iter(text)
                    .next()?
                    .get(0)?
                    .as_str()
                    .to_string(),
            )
        }

        fn get_current_version(cmd_name: &str) -> Result<String> {
            let output = std::process::Command::new(cmd_name)
                .arg("--version")
                .output()?;
            if !output.status.success() {
                return Err(anyhow!("Failed to run `{cmd_name} --version`"));
            }

            let output_version = std::str::from_utf8(&output.stdout)?;
            parse_version(output_version)
                .ok_or_else(|| anyhow!("Failed to parse the version of `{cmd_name}`"))
        }

        if let Some(solana_version) = &cfg.toolchain.solana_version {
            let current_version = get_current_version("solana")?;
            if solana_version != &current_version {
                // We are overriding with `solana-install` command instead of using the binaries
                // from `~/.local/share/solana/install/releases` because we use multiple Solana
                // binaries in various commands.
                fn override_solana_version(version: String) -> Result<bool> {
                    // There is a deprecation warning message starting with `1.18.19` which causes
                    // parsing problems https://github.com/coral-xyz/anchor/issues/3147
                    let (cmd_name, domain) =
                        if Version::parse(&version)? < Version::parse("1.18.19")? {
                            ("solana-install", "solana.com")
                        } else {
                            ("agave-install", "anza.xyz")
                        };

                    // Install the command if it's not installed
                    if get_current_version(cmd_name).is_err() {
                        // `solana-install` and `agave-install` are not usable at the same time i.e.
                        // using one of them makes the other unusable with the default installation,
                        // causing the installation process to run each time users switch between
                        // `agave` supported versions. For example, if the user's active Solana
                        // version is `1.18.17`, and he specifies `solana_version = "2.0.6"`, this
                        // code path will run each time an Anchor command gets executed.
                        eprintln!(
                            "Command not installed: `{cmd_name}`. \
                            See https://github.com/anza-xyz/agave/wiki/Agave-Transition, \
                            installing..."
                        );
                        let install_script = std::process::Command::new("curl")
                            .args([
                                "-sSfL",
                                &format!("https://release.{domain}/v{version}/install"),
                            ])
                            .output()?;
                        let is_successful = std::process::Command::new("sh")
                            .args(["-c", std::str::from_utf8(&install_script.stdout)?])
                            .spawn()?
                            .wait_with_output()?
                            .status
                            .success();
                        if !is_successful {
                            return Err(anyhow!("Failed to install `{cmd_name}`"));
                        }
                    }

                    let output = std::process::Command::new(cmd_name).arg("list").output()?;
                    if !output.status.success() {
                        return Err(anyhow!("Failed to list installed `solana` versions"));
                    }

                    // Hide the installation progress if the version is already installed
                    let is_installed = std::str::from_utf8(&output.stdout)?
                        .lines()
                        .filter_map(parse_version)
                        .any(|line_version| line_version == version);
                    let (stderr, stdout) = if is_installed {
                        (Stdio::null(), Stdio::null())
                    } else {
                        (Stdio::inherit(), Stdio::inherit())
                    };

                    std::process::Command::new(cmd_name)
                        .arg("init")
                        .arg(&version)
                        .stderr(stderr)
                        .stdout(stdout)
                        .spawn()?
                        .wait()
                        .map(|status| status.success())
                        .map_err(|err| anyhow!("Failed to run `{cmd_name}` command: {err}"))
                }

                match override_solana_version(solana_version.to_owned())? {
                    true => restore_cbs.push(Box::new(|| {
                        match override_solana_version(current_version)? {
                            true => Ok(()),
                            false => Err(anyhow!("Failed to restore `solana` version")),
                        }
                    })),
                    false => eprintln!(
                        "Failed to override `solana` version to {solana_version}, \
                        using {current_version} instead"
                    ),
                }
            }
        }

        // Anchor version override should be handled last
        if let Some(anchor_version) = &cfg.toolchain.anchor_version {
            // Anchor binary name prefix(applies to binaries that are installed via `avm`)
            const ANCHOR_BINARY_PREFIX: &str = "anchor-";

            // Get the current version from the executing binary name if possible because commit
            // based toolchain overrides do not have version information.
            let current_version = std::env::args()
                .next()
                .expect("First arg should exist")
                .parse::<PathBuf>()?
                .file_name()
                .and_then(|name| name.to_str())
                .expect("File name should be valid Unicode")
                .split_once(ANCHOR_BINARY_PREFIX)
                .map(|(_, version)| version)
                .unwrap_or(VERSION)
                .to_owned();
            if anchor_version != &current_version {
                let binary_path = home_dir()
                    .unwrap()
                    .join(".avm")
                    .join("bin")
                    .join(format!("{ANCHOR_BINARY_PREFIX}{anchor_version}"));

                if !binary_path.exists() {
                    eprintln!(
                        "`anchor` {anchor_version} is not installed with `avm`. Installing...\n"
                    );

                    if let Err(e) = install_with_avm(anchor_version, false) {
                        eprintln!(
                            "Failed to install `anchor`: {e}, using {current_version} instead"
                        );
                        return Ok(restore_cbs);
                    }
                }

                let exit_code = std::process::Command::new(binary_path)
                    .args(std::env::args_os().skip(1))
                    .spawn()?
                    .wait()?
                    .code()
                    .unwrap_or(1);
                restore_toolchain(restore_cbs)?;
                std::process::exit(exit_code);
            }
        }
    }

    Ok(restore_cbs)
}

/// Installs Anchor using AVM, passing `--force` (and optionally) installing
/// `solana-verify`.
fn install_with_avm(version: &str, verify: bool) -> Result<()> {
    let mut cmd = std::process::Command::new("avm");
    cmd.arg("install");
    cmd.arg(version);
    cmd.arg("--force");
    if verify {
        cmd.arg("--verify");
    }
    let status = cmd.status().context("running AVM")?;
    if !status.success() {
        bail!("failed to install `anchor` {version} with avm");
    }
    Ok(())
}

/// Restore toolchain to how it was before the command was run.
fn restore_toolchain(restore_cbs: RestoreToolchainCallbacks) -> Result<()> {
    for restore_toolchain in restore_cbs {
        if let Err(e) = restore_toolchain() {
            eprintln!("Toolchain error: {e}");
        }
    }

    Ok(())
}

/// Get the system's default license - what 'npm init' would use.
fn get_npm_init_license() -> Result<String> {
    let npm_init_license_output = std::process::Command::new("npm")
        .arg("config")
        .arg("get")
        .arg("init-license")
        .output()?;

    if !npm_init_license_output.status.success() {
        return Err(anyhow!("Failed to get npm init license"));
    }

    let license = String::from_utf8(npm_init_license_output.stdout)?;
    Ok(license.trim().to_string())
}

fn process_command(opts: Opts) -> Result<()> {
    match opts.command {
        Command::Init {
            name,
            javascript,
            no_install,
            package_manager,
            no_git,
            template,
            test_template,
            force,
        } => init(
            &opts.cfg_override,
            name,
            javascript,
            no_install,
            package_manager,
            no_git,
            template,
            test_template,
            force,
        ),
        Command::New {
            name,
            template,
            force,
        } => new(&opts.cfg_override, name, template, force),
        Command::Build {
            no_idl,
            idl,
            idl_ts,
            verifiable,
            program_name,
            solana_version,
            docker_image,
            bootstrap,
            cargo_args,
            env,
            skip_lint,
            ignore_keys,
            no_docs,
            arch,
        } => build(
            &opts.cfg_override,
            no_idl,
            idl,
            idl_ts,
            verifiable,
            skip_lint,
            ignore_keys,
            program_name,
            solana_version,
            docker_image,
            bootstrap,
            None,
            None,
            env,
            cargo_args,
            no_docs,
            arch,
        ),
        Command::Verify {
            program_id,
            repo_url,
            commit_hash,
            current_dir,
            program_name,
            args,
        } => verify(
            program_id,
            repo_url,
            commit_hash,
            current_dir,
            program_name,
            args,
        ),
        Command::Clean => clean(&opts.cfg_override),
        Command::Deploy {
            program_name,
            program_keypair,
            verifiable,
            no_idl,
            solana_args,
        } => deploy(
            &opts.cfg_override,
            program_name,
            program_keypair,
            verifiable,
            no_idl,
            solana_args,
        ),
        Command::Expand {
            program_name,
            cargo_args,
        } => expand(&opts.cfg_override, program_name, &cargo_args),
        Command::Upgrade {
            program_id,
            program_filepath,
            max_retries,
            solana_args,
        } => upgrade(
            &opts.cfg_override,
            program_id,
            program_filepath,
            max_retries,
            solana_args,
        ),
        Command::Idl { subcmd } => idl(&opts.cfg_override, subcmd),
        Command::Migrate => migrate(&opts.cfg_override),
        Command::Test {
            program_name,
            skip_deploy,
            skip_local_validator,
            skip_build,
            no_idl,
            detach,
            run,
            args,
            env,
            cargo_args,
            skip_lint,
            arch,
        } => test(
            &opts.cfg_override,
            program_name,
            skip_deploy,
            skip_local_validator,
            skip_build,
            skip_lint,
            no_idl,
            detach,
            run,
            args,
            env,
            cargo_args,
            arch,
        ),
        #[cfg(feature = "dev")]
        Command::Airdrop { .. } => airdrop(&opts.cfg_override),
        Command::Cluster { subcmd } => cluster(subcmd),
        Command::Shell => shell(&opts.cfg_override),
        Command::Run {
            script,
            script_args,
        } => run(&opts.cfg_override, script, script_args),
        Command::Login { token } => login(&opts.cfg_override, token),
        Command::Keys { subcmd } => keys(&opts.cfg_override, subcmd),
        Command::Localnet {
            skip_build,
            skip_deploy,
            skip_lint,
            ignore_keys,
            env,
            cargo_args,
            arch,
        } => localnet(
            &opts.cfg_override,
            skip_build,
            skip_deploy,
            skip_lint,
            ignore_keys,
            env,
            cargo_args,
            arch,
        ),
        Command::Account {
            account_type,
            address,
            idl,
        } => account(&opts.cfg_override, account_type, address, idl),
        Command::Completions { shell } => {
            clap_complete::generate(
                shell,
                &mut Opts::command(),
                "anchor",
                &mut std::io::stdout(),
            );
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn init(
    cfg_override: &ConfigOverride,
    name: String,
    javascript: bool,
    no_install: bool,
    package_manager: PackageManager,
    no_git: bool,
    template: ProgramTemplate,
    test_template: TestTemplate,
    force: bool,
) -> Result<()> {
    if !force && Config::discover(cfg_override)?.is_some() {
        return Err(anyhow!("Workspace already initialized"));
    }

    // We need to format different cases for the dir and the name
    let rust_name = name.to_snake_case();
    let project_name = if name == rust_name {
        rust_name.clone()
    } else {
        name.to_kebab_case()
    };

    // Additional keywords that have not been added to the `syn` crate as reserved words
    // https://github.com/dtolnay/syn/pull/1098
    let extra_keywords = ["async", "await", "try"];
    // Anchor converts to snake case before writing the program name
    if syn::parse_str::<syn::Ident>(&rust_name).is_err()
        || extra_keywords.contains(&rust_name.as_str())
    {
        return Err(anyhow!(
            "Anchor workspace name must be a valid Rust identifier. It may not be a Rust reserved word, start with a digit, or include certain disallowed characters. See https://doc.rust-lang.org/reference/identifiers.html for more detail.",
        ));
    }

    if force {
        fs::create_dir_all(&project_name)?;
    } else {
        fs::create_dir(&project_name)?;
    }
    std::env::set_current_dir(&project_name)?;
    fs::create_dir_all("app")?;

    let mut cfg = Config::default();

    let test_script = test_template.get_test_script(javascript, &package_manager);
    cfg.scripts.insert("test".to_owned(), test_script);

    let package_manager_cmd = package_manager.to_string();
    cfg.toolchain.package_manager = Some(package_manager);

    let mut localnet = BTreeMap::new();
    let program_id = rust_template::get_or_create_program_id(&rust_name);
    localnet.insert(
        rust_name,
        ProgramDeployment {
            address: program_id,
            path: None,
            idl: None,
        },
    );
    cfg.programs.insert(Cluster::Localnet, localnet);
    let toml = cfg.to_string();
    fs::write("Anchor.toml", toml)?;

    // Initialize .gitignore file
    fs::write(".gitignore", rust_template::git_ignore())?;

    // Initialize .prettierignore file
    fs::write(".prettierignore", rust_template::prettier_ignore())?;

    // Remove the default program if `--force` is passed
    if force {
        fs::remove_dir_all(
            std::env::current_dir()?
                .join("programs")
                .join(&project_name),
        )?;
    }

    // Build the program.
    rust_template::create_program(
        &project_name,
        template,
        TestTemplate::Mollusk == test_template,
    )?;

    // Build the migrations directory.
    let migrations_path = Path::new("migrations");
    fs::create_dir_all(migrations_path)?;

    let license = get_npm_init_license()?;

    let jest = TestTemplate::Jest == test_template;
    if javascript {
        // Build javascript config
        let mut package_json = File::create("package.json")?;
        package_json.write_all(rust_template::package_json(jest, license).as_bytes())?;

        let mut deploy = File::create(migrations_path.join("deploy.js"))?;
        deploy.write_all(rust_template::deploy_script().as_bytes())?;
    } else {
        // Build typescript config
        let mut ts_config = File::create("tsconfig.json")?;
        ts_config.write_all(rust_template::ts_config(jest).as_bytes())?;

        let mut ts_package_json = File::create("package.json")?;
        ts_package_json.write_all(rust_template::ts_package_json(jest, license).as_bytes())?;

        let mut deploy = File::create(migrations_path.join("deploy.ts"))?;
        deploy.write_all(rust_template::ts_deploy_script().as_bytes())?;
    }

    test_template.create_test_files(&project_name, javascript, &program_id.to_string())?;

    if !no_install {
        let package_manager_result = install_node_modules(&package_manager_cmd)?;

        if !package_manager_result.status.success() && package_manager_cmd != "npm" {
            println!("Failed {package_manager_cmd} install will attempt to npm install");
            install_node_modules("npm")?;
        } else {
            eprintln!("Failed to install node modules");
        }
    }

    if !no_git {
        let git_result = std::process::Command::new("git")
            .arg("init")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| anyhow::format_err!("git init failed: {}", e))?;
        if !git_result.status.success() {
            eprintln!("Failed to automatically initialize a new git repository");
        }
    }

    println!("{project_name} initialized");

    Ok(())
}

fn install_node_modules(cmd: &str) -> Result<std::process::Output> {
    if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .arg(format!("/C {cmd} install"))
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| anyhow::format_err!("{} install failed: {}", cmd, e))
    } else {
        std::process::Command::new(cmd)
            .arg("install")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| anyhow::format_err!("{} install failed: {}", cmd, e))
    }
}

// Creates a new program crate in the `programs/<name>` directory.
fn new(
    cfg_override: &ConfigOverride,
    name: String,
    template: ProgramTemplate,
    force: bool,
) -> Result<()> {
    with_workspace(cfg_override, |cfg| {
        match cfg.path().parent() {
            None => {
                println!("Unable to make new program");
            }
            Some(parent) => {
                std::env::set_current_dir(parent)?;

                let cluster = cfg.provider.cluster.clone();
                let programs = cfg.programs.entry(cluster).or_default();
                if programs.contains_key(&name) {
                    if !force {
                        return Err(anyhow!("Program already exists"));
                    }

                    // Delete all files within the program folder
                    fs::remove_dir_all(std::env::current_dir()?.join("programs").join(&name))?;
                }

                rust_template::create_program(&name, template, false)?;

                programs.insert(
                    name.clone(),
                    ProgramDeployment {
                        address: rust_template::get_or_create_program_id(&name),
                        path: None,
                        idl: None,
                    },
                );

                let toml = cfg.to_string();
                fs::write("Anchor.toml", toml)?;

                println!("Created new program.");
            }
        };
        Ok(())
    })
}

/// Array of (path, content) tuple.
pub type Files = Vec<(PathBuf, String)>;

/// Create files from the given (path, content) tuple array.
///
/// # Example
///
/// ```ignore
/// crate_files(vec![("programs/my_program/src/lib.rs".into(), "// Content".into())])?;
/// ```
pub fn create_files(files: &Files) -> Result<()> {
    for (path, content) in files {
        let path = path
            .display()
            .to_string()
            .replace('/', std::path::MAIN_SEPARATOR_STR);
        let path = Path::new(&path);
        if path.exists() {
            continue;
        }

        match path.extension() {
            Some(_) => {
                fs::create_dir_all(path.parent().unwrap())?;
                fs::write(path, content)?;
            }
            None => fs::create_dir_all(path)?,
        }
    }

    Ok(())
}

/// Override or create files from the given (path, content) tuple array.
///
/// # Example
///
/// ```ignore
/// override_or_create_files(vec![("programs/my_program/src/lib.rs".into(), "// Content".into())])?;
/// ```
pub fn override_or_create_files(files: &Files) -> Result<()> {
    for (path, content) in files {
        let path = Path::new(path);
        if path.exists() {
            let mut f = fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(path)?;
            f.write_all(content.as_bytes())?;
            f.flush()?;
        } else {
            fs::create_dir_all(path.parent().unwrap())?;
            fs::write(path, content)?;
        }
    }

    Ok(())
}

pub fn expand(
    cfg_override: &ConfigOverride,
    program_name: Option<String>,
    cargo_args: &[String],
) -> Result<()> {
    // Change to the workspace member directory, if needed.
    if let Some(program_name) = program_name.as_ref() {
        cd_member(cfg_override, program_name)?;
    }

    let workspace_cfg = Config::discover(cfg_override)?.expect("Not in workspace.");
    let cfg_parent = workspace_cfg.path().parent().expect("Invalid Anchor.toml");
    let cargo = Manifest::discover()?;

    let expansions_path = cfg_parent.join(".anchor").join("expanded-macros");
    fs::create_dir_all(&expansions_path)?;

    match cargo {
        // No Cargo.toml found, expand entire workspace
        None => expand_all(&workspace_cfg, expansions_path, cargo_args),
        // Cargo.toml is at root of workspace, expand entire workspace
        Some(cargo) if cargo.path().parent() == workspace_cfg.path().parent() => {
            expand_all(&workspace_cfg, expansions_path, cargo_args)
        }
        // Reaching this arm means Cargo.toml belongs to a single package. Expand it.
        Some(cargo) => expand_program(
            // If we found Cargo.toml, it must be in a directory so unwrap is safe
            cargo.path().parent().unwrap().to_path_buf(),
            expansions_path,
            cargo_args,
        ),
    }
}

fn expand_all(
    workspace_cfg: &WithPath<Config>,
    expansions_path: PathBuf,
    cargo_args: &[String],
) -> Result<()> {
    let cur_dir = std::env::current_dir()?;
    for p in workspace_cfg.get_rust_program_list()? {
        expand_program(p, expansions_path.clone(), cargo_args)?;
    }
    std::env::set_current_dir(cur_dir)?;
    Ok(())
}

fn expand_program(
    program_path: PathBuf,
    expansions_path: PathBuf,
    cargo_args: &[String],
) -> Result<()> {
    let cargo = Manifest::from_path(program_path.join("Cargo.toml"))
        .map_err(|_| anyhow!("Could not find Cargo.toml for program"))?;

    let target_dir_arg = {
        let mut target_dir_arg = OsString::from("--target-dir=");
        target_dir_arg.push(expansions_path.join("expand-target"));
        target_dir_arg
    };

    let package_name = &cargo
        .package
        .as_ref()
        .ok_or_else(|| anyhow!("Cargo config is missing a package"))?
        .name;
    let program_expansions_path = expansions_path.join(package_name);
    fs::create_dir_all(&program_expansions_path)?;

    let exit = std::process::Command::new("cargo")
        .arg("expand")
        .arg(target_dir_arg)
        .arg(format!("--package={package_name}"))
        .args(cargo_args)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| anyhow::format_err!("{}", e))?;
    if !exit.status.success() {
        eprintln!("'anchor expand' failed. Perhaps you have not installed 'cargo-expand'? https://github.com/dtolnay/cargo-expand#installation");
        std::process::exit(exit.status.code().unwrap_or(1));
    }

    let version = cargo.version();
    let time = chrono::Utc::now().to_string().replace(' ', "_");
    let file_path = program_expansions_path.join(format!("{package_name}-{version}-{time}.rs"));
    fs::write(&file_path, &exit.stdout).map_err(|e| anyhow::format_err!("{}", e))?;

    println!(
        "Expanded {} into file {}\n",
        package_name,
        file_path.to_string_lossy()
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn build(
    cfg_override: &ConfigOverride,
    no_idl: bool,
    idl: Option<String>,
    idl_ts: Option<String>,
    verifiable: bool,
    skip_lint: bool,
    ignore_keys: bool,
    program_name: Option<String>,
    solana_version: Option<String>,
    docker_image: Option<String>,
    bootstrap: BootstrapMode,
    stdout: Option<File>, // Used for the package registry server.
    stderr: Option<File>, // Used for the package registry server.
    env_vars: Vec<String>,
    cargo_args: Vec<String>,
    no_docs: bool,
    arch: ProgramArch,
) -> Result<()> {
    // Change to the workspace member directory, if needed.
    if let Some(program_name) = program_name.as_ref() {
        cd_member(cfg_override, program_name)?;
    }
    let cfg = Config::discover(cfg_override)?.expect("Not in workspace.");
    let cfg_parent = cfg.path().parent().expect("Invalid Anchor.toml");

    // Require overflow checks
    let workspace_cargo_toml_path = cfg_parent.join("Cargo.toml");
    if workspace_cargo_toml_path.exists() {
        check_overflow(workspace_cargo_toml_path)?;
    }

    // Check whether there is a mismatch between CLI and crate/package versions
    check_anchor_version(&cfg).ok();
    check_deps(&cfg).ok();

    // Check for program ID mismatches before building (skip if --ignore-keys is used), Always skipped in anchor test
    if !ignore_keys {
        check_program_id_mismatch(&cfg, program_name.clone())?;
    }

    let idl_out = match idl {
        Some(idl) => Some(PathBuf::from(idl)),
        None => Some(cfg_parent.join("target").join("idl")),
    };
    fs::create_dir_all(idl_out.as_ref().unwrap())?;

    let idl_ts_out = match idl_ts {
        Some(idl_ts) => Some(PathBuf::from(idl_ts)),
        None => Some(cfg_parent.join("target").join("types")),
    };
    fs::create_dir_all(idl_ts_out.as_ref().unwrap())?;

    if !cfg.workspace.types.is_empty() {
        fs::create_dir_all(cfg_parent.join(&cfg.workspace.types))?;
    };

    cfg.run_hooks(HookType::PreBuild)?;

    let cargo = Manifest::discover()?;
    let build_config = BuildConfig {
        verifiable,
        solana_version: solana_version.or_else(|| cfg.toolchain.solana_version.clone()),
        docker_image: docker_image.unwrap_or_else(|| cfg.docker()),
        bootstrap,
    };
    match cargo {
        // No Cargo.toml so build the entire workspace.
        None => build_all(
            &cfg,
            cfg.path(),
            no_idl,
            idl_out,
            idl_ts_out,
            &build_config,
            stdout,
            stderr,
            env_vars,
            cargo_args,
            skip_lint,
            no_docs,
            arch,
        )?,
        // If the Cargo.toml is at the root, build the entire workspace.
        Some(cargo) if cargo.path().parent() == cfg.path().parent() => build_all(
            &cfg,
            cfg.path(),
            no_idl,
            idl_out,
            idl_ts_out,
            &build_config,
            stdout,
            stderr,
            env_vars,
            cargo_args,
            skip_lint,
            no_docs,
            arch,
        )?,
        // Cargo.toml represents a single package. Build it.
        Some(cargo) => build_rust_cwd(
            &cfg,
            cargo.path().to_path_buf(),
            no_idl,
            idl_out,
            idl_ts_out,
            &build_config,
            stdout,
            stderr,
            env_vars,
            cargo_args,
            skip_lint,
            no_docs,
            &arch,
        )?,
    }
    cfg.run_hooks(HookType::PostBuild)?;

    set_workspace_dir_or_exit();

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn build_all(
    cfg: &WithPath<Config>,
    cfg_path: &Path,
    no_idl: bool,
    idl_out: Option<PathBuf>,
    idl_ts_out: Option<PathBuf>,
    build_config: &BuildConfig,
    stdout: Option<File>, // Used for the package registry server.
    stderr: Option<File>, // Used for the package registry server.
    env_vars: Vec<String>,
    cargo_args: Vec<String>,
    skip_lint: bool,
    no_docs: bool,
    arch: ProgramArch,
) -> Result<()> {
    let cur_dir = std::env::current_dir()?;
    let r = match cfg_path.parent() {
        None => Err(anyhow!("Invalid Anchor.toml at {}", cfg_path.display())),
        Some(_parent) => {
            for p in cfg.get_rust_program_list()? {
                build_rust_cwd(
                    cfg,
                    p.join("Cargo.toml"),
                    no_idl,
                    idl_out.clone(),
                    idl_ts_out.clone(),
                    build_config,
                    stdout.as_ref().map(|f| f.try_clone()).transpose()?,
                    stderr.as_ref().map(|f| f.try_clone()).transpose()?,
                    env_vars.clone(),
                    cargo_args.clone(),
                    skip_lint,
                    no_docs,
                    &arch,
                )?;
            }
            Ok(())
        }
    };
    std::env::set_current_dir(cur_dir)?;
    r
}

// Runs the build command outside of a workspace.
#[allow(clippy::too_many_arguments)]
fn build_rust_cwd(
    cfg: &WithPath<Config>,
    cargo_toml: PathBuf,
    no_idl: bool,
    idl_out: Option<PathBuf>,
    idl_ts_out: Option<PathBuf>,
    build_config: &BuildConfig,
    stdout: Option<File>,
    stderr: Option<File>,
    env_vars: Vec<String>,
    cargo_args: Vec<String>,
    skip_lint: bool,
    no_docs: bool,
    arch: &ProgramArch,
) -> Result<()> {
    match cargo_toml.parent() {
        None => return Err(anyhow!("Unable to find parent")),
        Some(p) => std::env::set_current_dir(p)?,
    };
    match build_config.verifiable {
        false => _build_rust_cwd(
            cfg, no_idl, idl_out, idl_ts_out, skip_lint, no_docs, arch, cargo_args,
        ),
        true => build_cwd_verifiable(
            cfg,
            cargo_toml,
            build_config,
            stdout,
            stderr,
            skip_lint,
            env_vars,
            cargo_args,
            no_docs,
            arch,
        ),
    }
}

// Builds an anchor program in a docker image and copies the build artifacts
// into the `target/` directory.
#[allow(clippy::too_many_arguments)]
fn build_cwd_verifiable(
    cfg: &WithPath<Config>,
    cargo_toml: PathBuf,
    build_config: &BuildConfig,
    stdout: Option<File>,
    stderr: Option<File>,
    skip_lint: bool,
    env_vars: Vec<String>,
    cargo_args: Vec<String>,
    no_docs: bool,
    arch: &ProgramArch,
) -> Result<()> {
    // Create output dirs.
    let workspace_dir = cfg.path().parent().unwrap().canonicalize()?;
    let target_dir = workspace_dir.join("target");
    fs::create_dir_all(target_dir.join("verifiable"))?;
    fs::create_dir_all(target_dir.join("idl"))?;
    fs::create_dir_all(target_dir.join("types"))?;
    if !&cfg.workspace.types.is_empty() {
        fs::create_dir_all(workspace_dir.join(&cfg.workspace.types))?;
    }

    let container_name = "anchor-program";

    // Build the binary in docker.
    let result = docker_build(
        cfg,
        container_name,
        cargo_toml,
        build_config,
        stdout,
        stderr,
        env_vars,
        cargo_args.clone(),
        arch,
    );

    match &result {
        Err(e) => {
            eprintln!("Error during Docker build: {e:?}");
        }
        Ok(_) => {
            // Build the idl.
            println!("Extracting the IDL");
            let idl = generate_idl(cfg, skip_lint, no_docs, &cargo_args)?;
            // Write out the JSON file.
            println!("Writing the IDL file");
            let out_file = workspace_dir
                .join("target")
                .join("idl")
                .join(&idl.metadata.name)
                .with_extension("json");
            write_idl(&idl, OutFile::File(out_file))?;

            // Write out the TypeScript type.
            println!("Writing the .ts file");
            let ts_file = workspace_dir
                .join("target")
                .join("types")
                .join(&idl.metadata.name)
                .with_extension("ts");
            fs::write(&ts_file, idl_ts(&idl)?)?;

            // Copy out the TypeScript type.
            if !&cfg.workspace.types.is_empty() {
                fs::copy(
                    ts_file,
                    workspace_dir
                        .join(&cfg.workspace.types)
                        .join(idl.metadata.name)
                        .with_extension("ts"),
                )?;
            }

            println!("Build success");
        }
    }

    result
}

#[allow(clippy::too_many_arguments)]
fn docker_build(
    cfg: &WithPath<Config>,
    container_name: &str,
    cargo_toml: PathBuf,
    build_config: &BuildConfig,
    stdout: Option<File>,
    stderr: Option<File>,
    env_vars: Vec<String>,
    cargo_args: Vec<String>,
    arch: &ProgramArch,
) -> Result<()> {
    let binary_name = Manifest::from_path(&cargo_toml)?.lib_name()?;

    // Docker vars.
    let workdir = Path::new("/workdir");
    let volume_mount = format!(
        "{}:{}",
        cfg.path().parent().unwrap().canonicalize()?.display(),
        workdir.to_str().unwrap(),
    );
    println!("Using image {:?}", build_config.docker_image);

    // Start the docker image running detached in the background.
    let target_dir = workdir.join("docker-target");
    println!("Run docker image");
    let exit = std::process::Command::new("docker")
        .args([
            "run",
            "-it",
            "-d",
            "--name",
            container_name,
            "--env",
            &format!(
                "CARGO_TARGET_DIR={}",
                target_dir.as_path().to_str().unwrap()
            ),
            "-v",
            &volume_mount,
            "-w",
            workdir.to_str().unwrap(),
            &build_config.docker_image,
            "bash",
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| anyhow::format_err!("Docker build failed: {}", e))?;
    if !exit.status.success() {
        return Err(anyhow!("Failed to build program"));
    }

    let result = docker_prep(container_name, build_config).and_then(|_| {
        let cfg_parent = cfg.path().parent().unwrap();
        docker_build_bpf(
            container_name,
            cargo_toml.as_path(),
            cfg_parent,
            target_dir.as_path(),
            binary_name,
            stdout,
            stderr,
            env_vars,
            cargo_args,
            arch,
        )
    });

    // Cleanup regardless of errors
    docker_cleanup(container_name, target_dir.as_path())?;

    // Done.
    result
}

fn docker_prep(container_name: &str, build_config: &BuildConfig) -> Result<()> {
    // Set the solana version in the container, if given. Otherwise use the
    // default.
    match build_config.bootstrap {
        BootstrapMode::Debian => {
            // Install build requirements
            docker_exec(container_name, &["apt", "update"])?;
            docker_exec(
                container_name,
                &["apt", "install", "-y", "curl", "build-essential"],
            )?;

            // Install Rust
            docker_exec(
                container_name,
                &["curl", "https://sh.rustup.rs", "-sfo", "rustup.sh"],
            )?;
            docker_exec(container_name, &["sh", "rustup.sh", "-y"])?;
            docker_exec(container_name, &["rm", "-f", "rustup.sh"])?;
        }
        BootstrapMode::None => {}
    }

    if let Some(solana_version) = &build_config.solana_version {
        println!("Using solana version: {solana_version}");

        // Install Solana CLI
        docker_exec(
            container_name,
            &[
                "curl",
                "-sSfL",
                &format!("https://release.anza.xyz/v{solana_version}/install",),
                "-o",
                "solana_installer.sh",
            ],
        )?;
        docker_exec(container_name, &["sh", "solana_installer.sh"])?;
        docker_exec(container_name, &["rm", "-f", "solana_installer.sh"])?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn docker_build_bpf(
    container_name: &str,
    cargo_toml: &Path,
    cfg_parent: &Path,
    target_dir: &Path,
    binary_name: String,
    stdout: Option<File>,
    stderr: Option<File>,
    env_vars: Vec<String>,
    cargo_args: Vec<String>,
    arch: &ProgramArch,
) -> Result<()> {
    let manifest_path =
        pathdiff::diff_paths(cargo_toml.canonicalize()?, cfg_parent.canonicalize()?)
            .ok_or_else(|| anyhow!("Unable to diff paths"))?;
    println!(
        "Building {} manifest: {:?}",
        binary_name,
        manifest_path.display()
    );

    // Execute the build.
    let exit = std::process::Command::new("docker")
        .args([
            "exec",
            "--env",
            "PATH=/root/.local/share/solana/install/active_release/bin:/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        ])
        .args(env_vars
            .iter()
            .map(|x| ["--env", x.as_str()])
            .collect::<Vec<[&str; 2]>>()
            .concat())
        .args([
            container_name,
            "cargo",
        ])
        .args(arch.build_subcommand())
        .args([
            "--manifest-path",
            &manifest_path.display().to_string(),
        ])
        .args(cargo_args)
        .stdout(match stdout {
            None => Stdio::inherit(),
            Some(f) => f.into(),
        })
        .stderr(match stderr {
            None => Stdio::inherit(),
            Some(f) => f.into(),
        })
        .output()
        .map_err(|e| anyhow::format_err!("Docker build failed: {}", e))?;
    if !exit.status.success() {
        return Err(anyhow!("Failed to build program"));
    }

    // Copy the binary out of the docker image.
    println!("Copying out the build artifacts");
    let out_file = cfg_parent
        .canonicalize()?
        .join(
            Path::new("target")
                .join("verifiable")
                .join(&binary_name)
                .with_extension("so"),
        )
        .display()
        .to_string();

    // This requires the target directory of any built program to be located at
    // the root of the workspace.
    let mut bin_path = target_dir.join("deploy");
    bin_path.push(format!("{binary_name}.so"));
    let bin_artifact = format!(
        "{}:{}",
        container_name,
        bin_path.as_path().to_str().unwrap()
    );
    let exit = std::process::Command::new("docker")
        .args(["cp", &bin_artifact, &out_file])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| anyhow::format_err!("{}", e))?;
    if !exit.status.success() {
        Err(anyhow!(
            "Failed to copy binary out of docker. Is the target directory set correctly?"
        ))
    } else {
        Ok(())
    }
}

fn docker_cleanup(container_name: &str, target_dir: &Path) -> Result<()> {
    // Wipe the generated docker-target dir.
    println!("Cleaning up the docker target directory");
    docker_exec(container_name, &["rm", "-rf", target_dir.to_str().unwrap()])?;

    // Remove the docker image.
    println!("Removing the docker container");
    let exit = std::process::Command::new("docker")
        .args(["rm", "-f", container_name])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| anyhow::format_err!("{}", e))?;
    if !exit.status.success() {
        println!("Unable to remove the docker container");
        std::process::exit(exit.status.code().unwrap_or(1));
    }
    Ok(())
}

fn docker_exec(container_name: &str, args: &[&str]) -> Result<()> {
    let exit = std::process::Command::new("docker")
        .args([&["exec", container_name], args].concat())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| anyhow!("Failed to run command \"{:?}\": {:?}", args, e))?;
    if !exit.status.success() {
        Err(anyhow!("Failed to run command: {:?}", args))
    } else {
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn _build_rust_cwd(
    cfg: &WithPath<Config>,
    no_idl: bool,
    idl_out: Option<PathBuf>,
    idl_ts_out: Option<PathBuf>,
    skip_lint: bool,
    no_docs: bool,
    arch: &ProgramArch,
    cargo_args: Vec<String>,
) -> Result<()> {
    let exit = std::process::Command::new("cargo")
        .args(arch.build_subcommand())
        .args(cargo_args.clone())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| anyhow::format_err!("{}", e))?;
    if !exit.status.success() {
        std::process::exit(exit.status.code().unwrap_or(1));
    }

    // Generate IDL
    if !no_idl {
        let idl = generate_idl(cfg, skip_lint, no_docs, &cargo_args)?;

        // JSON out path.
        let out = match idl_out {
            None => PathBuf::from(".")
                .join(&idl.metadata.name)
                .with_extension("json"),
            Some(o) => PathBuf::from(&o.join(&idl.metadata.name).with_extension("json")),
        };
        // TS out path.
        let ts_out = match idl_ts_out {
            None => PathBuf::from(".")
                .join(&idl.metadata.name)
                .with_extension("ts"),
            Some(o) => PathBuf::from(&o.join(&idl.metadata.name).with_extension("ts")),
        };

        // Write out the JSON file.
        write_idl(&idl, OutFile::File(out))?;
        // Write out the TypeScript type.
        fs::write(&ts_out, idl_ts(&idl)?)?;

        // Copy out the TypeScript type.
        let cfg_parent = cfg.path().parent().expect("Invalid Anchor.toml");
        if !&cfg.workspace.types.is_empty() {
            fs::copy(
                &ts_out,
                cfg_parent
                    .join(&cfg.workspace.types)
                    .join(&idl.metadata.name)
                    .with_extension("ts"),
            )?;
        }
    }

    Ok(())
}

pub fn verify(
    program_id: Pubkey,
    repo_url: Option<String>,
    commit_hash: Option<String>,
    current_dir: bool,
    program_name: Option<String>,
    args: Vec<String>,
) -> Result<()> {
    let mut command_args = Vec::new();

    match (current_dir, repo_url) {
        (true, _) => {
            let current_path = std::env::current_dir()?
                .to_str()
                .ok_or_else(|| anyhow!("Invalid current directory path"))?
                .to_owned();
            command_args.push(current_path);
            command_args.push("--current-dir".into());
        }
        (false, Some(url)) => {
            command_args.push(url);
        }
        (false, None) => {
            return Err(anyhow!(
                "You must provide either --repo-url or --current-dir"
            ));
        }
    }

    if let Some(commit) = commit_hash {
        command_args.push("--commit-hash".into());
        command_args.push(commit);
    }

    if let Some(name) = program_name {
        command_args.push("--library-name".into());
        command_args.push(name);
    }

    command_args.push("--program-id".into());
    command_args.push(program_id.to_string());

    command_args.extend(args);

    println!("Verifying program {program_id}");
    let verify_path = AVM_HOME.join("bin").join("solana-verify");
    if !verify_path.exists() {
        install_with_avm(env!("CARGO_PKG_VERSION"), true)
            .context("installing Anchor with solana-verify")?;
    }

    let status = std::process::Command::new(verify_path)
        .arg("verify-from-repo")
        .args(&command_args)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .with_context(|| "Failed to run `solana-verify`")?;

    if !status.success() {
        return Err(anyhow!("Failed to verify program"));
    }

    Ok(())
}

fn cd_member(cfg_override: &ConfigOverride, program_name: &str) -> Result<()> {
    // Change directories to the given `program_name`, if given.
    let cfg = Config::discover(cfg_override)?.expect("Not in workspace.");

    for program in cfg.read_all_programs()? {
        let cargo_toml = program.path.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Err(anyhow!(
                "Did not find Cargo.toml at the path: {}",
                program.path.display()
            ));
        }

        let manifest = Manifest::from_path(&cargo_toml)?;
        let pkg_name = manifest.package().name();
        let lib_name = manifest.lib_name()?;
        if program_name == pkg_name || program_name == lib_name {
            std::env::set_current_dir(&program.path)?;
            return Ok(());
        }
    }

    Err(anyhow!("{} is not part of the workspace", program_name,))
}

fn idl(cfg_override: &ConfigOverride, subcmd: IdlCommand) -> Result<()> {
    match subcmd {
        IdlCommand::Init {
            program_id,
            filepath,
            priority_fee,
            non_canonical,
        } => idl_init(
            cfg_override,
            program_id,
            filepath,
            priority_fee,
            non_canonical,
        ),
        IdlCommand::Upgrade {
            program_id,
            filepath,
            priority_fee,
        } => idl_upgrade(cfg_override, program_id, filepath, priority_fee),
        IdlCommand::Build {
            program_name,
            out,
            out_ts,
            no_docs,
            skip_lint,
            cargo_args,
        } => idl_build(
            cfg_override,
            program_name,
            out,
            out_ts,
            no_docs,
            skip_lint,
            cargo_args,
        ),
        IdlCommand::Fetch {
            address,
            out,
            non_canonical,
        } => idl_fetch(cfg_override, address, out, non_canonical),
        IdlCommand::Convert {
            path,
            out,
            program_id,
        } => idl_convert(path, out, program_id),
        IdlCommand::Type { path, out } => idl_type(path, out),
        IdlCommand::Close {
            program_id,
            seed,
            priority_fee,
        } => idl_close_metadata(cfg_override, program_id, seed, priority_fee),
        IdlCommand::CreateBuffer {
            filepath,
            priority_fee,
        } => idl_create_buffer(cfg_override, filepath, priority_fee),
        IdlCommand::SetBufferAuthority {
            buffer,
            new_authority,
            priority_fee,
        } => idl_set_buffer_authority(cfg_override, buffer, new_authority, priority_fee),
        IdlCommand::WriteBuffer {
            program_id,
            buffer,
            seed,
            close_buffer,
            priority_fee,
        } => idl_write_buffer_metadata(
            cfg_override,
            program_id,
            buffer,
            seed,
            close_buffer,
            priority_fee,
        ),
    }
}

fn rpc_url(cfg_override: &ConfigOverride) -> Result<String> {
    let cfg = Config::discover(cfg_override)?.expect("Not in workspace");
    Ok(cluster_url(&cfg, &cfg.test_validator))
}

fn idl_init(
    cfg_override: &ConfigOverride,
    program_id: Pubkey,
    idl_filepath: String,
    priority_fee: Option<u64>,
    non_canonical: bool,
) -> Result<()> {
    let url = rpc_url(cfg_override)?;

    let program_id_str = program_id.to_string();
    let mut args = vec!["write", "idl", &program_id_str, &idl_filepath];

    if non_canonical {
        args.push("--non-canonical");
    }

    let priority_fee_str;
    if let Some(priority_fee) = priority_fee {
        priority_fee_str = priority_fee.to_string();
        args.push("--priority-fees");
        args.push(&priority_fee_str);
    }

    args.push("--rpc");
    args.push(&url);

    let status = ProcessCommand::new("npx")
        .arg("@solana-program/program-metadata")
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(anyhow!("Failed to initialize IDL"));
    }

    println!("IDL initialized.");
    Ok(())
}

fn idl_upgrade(
    cfg_override: &ConfigOverride,
    program_id: Pubkey,
    idl_filepath: String,
    priority_fee: Option<u64>,
) -> Result<()> {
    let url = rpc_url(cfg_override)?;

    let program_id_str = program_id.to_string();
    let mut args = vec!["write", "idl", &program_id_str, &idl_filepath];

    let priority_fee_str;
    if let Some(priority_fee) = priority_fee {
        priority_fee_str = priority_fee.to_string();
        args.push("--priority-fees");
        args.push(&priority_fee_str);
    }

    args.push("--rpc");
    args.push(&url);

    let status = ProcessCommand::new("npx")
        .arg("@solana-program/program-metadata")
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(anyhow!("Failed to upgrade IDL"));
    }

    println!("IDL upgraded.");
    Ok(())
}

fn idl_build(
    cfg_override: &ConfigOverride,
    program_name: Option<String>,
    out: Option<String>,
    out_ts: Option<String>,
    no_docs: bool,
    skip_lint: bool,
    cargo_args: Vec<String>,
) -> Result<()> {
    let cfg = Config::discover(cfg_override)?.expect("Not in workspace");
    let current_dir = std::env::current_dir()?;
    let program_path = match program_name {
        Some(name) => cfg.get_program(&name)?.path,
        None => {
            let programs = cfg.read_all_programs()?;
            if programs.len() == 1 {
                programs.into_iter().next().unwrap().path
            } else {
                programs
                    .into_iter()
                    .find(|program| program.path == current_dir)
                    .ok_or_else(|| anyhow!("Not in a program directory"))?
                    .path
            }
        }
    };
    std::env::set_current_dir(program_path)?;
    let idl = generate_idl(&cfg, skip_lint, no_docs, &cargo_args)?;
    std::env::set_current_dir(current_dir)?;

    let out = match out {
        Some(path) => OutFile::File(PathBuf::from(path)),
        None => OutFile::Stdout,
    };
    write_idl(&idl, out)?;

    if let Some(path) = out_ts {
        fs::write(path, idl_ts(&idl)?)?;
    }

    Ok(())
}

/// Generate IDL with method decided by whether manifest file has `idl-build` feature or not.
fn generate_idl(
    cfg: &WithPath<Config>,
    skip_lint: bool,
    no_docs: bool,
    cargo_args: &[String],
) -> Result<Idl> {
    check_idl_build_feature()?;

    anchor_lang_idl::build::IdlBuilder::new()
        .resolution(cfg.features.resolution)
        .skip_lint(cfg.features.skip_lint || skip_lint)
        .no_docs(no_docs)
        .cargo_args(cargo_args.into())
        .build()
}

fn idl_fetch(
    cfg_override: &ConfigOverride,
    address: Pubkey,
    out: Option<String>,
    non_canonical: bool,
) -> Result<()> {
    // The address parameter is the program ID
    // Program Metadata CLI expects: fetch <seed> <program>
    // For IDL, the seed is "idl"
    let program_id_str = address.to_string();
    let mut args = vec!["fetch", "idl", &program_id_str];

    if non_canonical {
        args.push("--non-canonical");
    }

    if let Some(out) = &out {
        args.push("-o");
        args.push(out);
    }
    let url = rpc_url(cfg_override)?;
    args.push("--rpc");
    args.push(&url);

    let status = ProcessCommand::new("npx")
        .arg("@solana-program/program-metadata")
        .args(&args)
        .status()?;

    if !status.success() {
        return Err(anyhow!("Failed to fetch IDL"));
    }
    Ok(())
}

fn idl_convert(path: String, out: Option<String>, program_id: Option<Pubkey>) -> Result<()> {
    let idl = fs::read(path)?;

    // Set the `metadata.address` field based on the given `program_id`
    let idl = match program_id {
        Some(program_id) => {
            let mut idl = serde_json::from_slice::<serde_json::Value>(&idl)?;
            idl.as_object_mut()
                .ok_or_else(|| anyhow!("IDL must be an object"))?
                .insert(
                    "metadata".into(),
                    serde_json::json!({ "address": program_id.to_string() }),
                );
            serde_json::to_vec(&idl)?
        }
        _ => idl,
    };

    let idl = convert_idl(&idl)?;
    let out = match out {
        None => OutFile::Stdout,
        Some(out) => OutFile::File(PathBuf::from(out)),
    };
    write_idl(&idl, out)
}

fn idl_type(path: String, out: Option<String>) -> Result<()> {
    let idl = fs::read(path)?;
    let idl = convert_idl(&idl)?;
    let types = idl_ts(&idl)?;
    match out {
        Some(out) => fs::write(out, types)?,
        _ => println!("{types}"),
    };
    Ok(())
}

fn idl_close_metadata(
    cfg_override: &ConfigOverride,
    program_id: Pubkey,
    seed: String,
    priority_fee: Option<u64>,
) -> Result<()> {
    let program_id_str = program_id.to_string();
    let mut args = vec!["close", &seed, &program_id_str];

    let priority_fee_str;
    if let Some(priority_fee) = priority_fee {
        priority_fee_str = priority_fee.to_string();
        args.push("--priority-fees");
        args.push(&priority_fee_str);
    }

    let url = rpc_url(cfg_override)?;
    args.push("--rpc");
    args.push(&url);

    let status = ProcessCommand::new("npx")
        .arg("@solana-program/program-metadata")
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(anyhow!("Failed to close metadata account"));
    }

    println!("Metadata account closed successfully.");
    Ok(())
}

fn idl_create_buffer(
    cfg_override: &ConfigOverride,
    filepath: String,
    priority_fee: Option<u64>,
) -> Result<()> {
    let mut args = vec!["create-buffer", &filepath];

    let priority_fee_str;
    if let Some(priority_fee) = priority_fee {
        priority_fee_str = priority_fee.to_string();
        args.push("--priority-fees");
        args.push(&priority_fee_str);
    }

    let url = rpc_url(cfg_override)?;
    args.push("--rpc");
    args.push(&url);

    let status = ProcessCommand::new("npx")
        .arg("@solana-program/program-metadata")
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(anyhow!("Failed to create buffer"));
    }

    println!("Buffer created successfully.");
    Ok(())
}

fn idl_set_buffer_authority(
    cfg_override: &ConfigOverride,
    buffer: Pubkey,
    new_authority: Pubkey,
    priority_fee: Option<u64>,
) -> Result<()> {
    let buffer_str = buffer.to_string();
    let new_authority_str = new_authority.to_string();
    let mut args = vec![
        "set-buffer-authority",
        &buffer_str,
        "--new-authority",
        &new_authority_str,
    ];

    let priority_fee_str;
    if let Some(priority_fee) = priority_fee {
        priority_fee_str = priority_fee.to_string();
        args.push("--priority-fees");
        args.push(&priority_fee_str);
    }

    let url = rpc_url(cfg_override)?;
    args.push("--rpc");
    args.push(&url);

    let status = ProcessCommand::new("npx")
        .arg("@solana-program/program-metadata")
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(anyhow!("Failed to set buffer authority"));
    }

    println!("Buffer authority set successfully.");
    Ok(())
}

fn idl_write_buffer_metadata(
    cfg_override: &ConfigOverride,
    program_id: Pubkey,
    buffer: Pubkey,
    seed: String,
    close_buffer: bool,
    priority_fee: Option<u64>,
) -> Result<()> {
    let program_id_str = program_id.to_string();
    let buffer_str = buffer.to_string();
    let mut args = vec!["write", &seed, &program_id_str, "--buffer", &buffer_str];

    if close_buffer {
        args.push("--close-buffer");
    }

    let priority_fee_str;
    if let Some(priority_fee) = priority_fee {
        priority_fee_str = priority_fee.to_string();
        args.push("--priority-fees");
        args.push(&priority_fee_str);
    }

    let url = rpc_url(cfg_override)?;
    args.push("--rpc");
    args.push(&url);

    let status = ProcessCommand::new("npx")
        .arg("@solana-program/program-metadata")
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(anyhow!("Failed to write metadata using buffer"));
    }

    println!("Metadata written successfully using buffer.");
    Ok(())
}

fn idl_ts(idl: &Idl) -> Result<String> {
    let idl_name = &idl.metadata.name;
    let type_name = idl_name.to_pascal_case();
    let idl = serde_json::to_string(idl)?;

    // Convert every field of the IDL to camelCase
    let camel_idl = Regex::new(r#""\w+":"([\w\d]+)""#)?
        .captures_iter(&idl)
        .fold(idl.clone(), |acc, cur| {
            let name = cur.get(1).unwrap().as_str();

            // Do not modify pubkeys
            if Pubkey::try_from(name).is_ok() {
                return acc;
            }

            let camel_name = name.to_lower_camel_case();
            acc.replace(&format!(r#""{name}""#), &format!(r#""{camel_name}""#))
        });

    // Pretty format
    let camel_idl = serde_json::to_string_pretty(&serde_json::from_str::<Idl>(&camel_idl)?)?;

    Ok(format!(
        r#"/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/{idl_name}.json`.
 */
export type {type_name} = {camel_idl};
"#
    ))
}

fn write_idl(idl: &Idl, out: OutFile) -> Result<()> {
    let idl_json = serde_json::to_string_pretty(idl)?;
    match out {
        OutFile::Stdout => println!("{idl_json}"),
        OutFile::File(out) => fs::write(out, idl_json)?,
    };

    Ok(())
}
fn account(
    cfg_override: &ConfigOverride,
    account_type: String,
    address: Pubkey,
    idl_filepath: Option<String>,
) -> Result<()> {
    let (program_name, account_type_name) = account_type
        .split_once('.') // Split at first occurrence of dot
        .and_then(|(x, y)| y.find('.').map_or_else(|| Some((x, y)), |_| None)) // ensures no dots in second substring
        .ok_or_else(|| {
            anyhow!(
                "Please enter the account struct in the following format: <program_name>.<Account>",
            )
        })?;

    let idl = idl_filepath.map_or_else(
        || {
            Config::discover(cfg_override)
                .expect("Error when detecting workspace.")
                .expect("Not in workspace.")
                .read_all_programs()
                .expect("Workspace must contain atleast one program.")
                .into_iter()
                .find(|p| p.lib_name == *program_name)
                .ok_or_else(|| anyhow!("Program {program_name} not found in workspace."))
                .map(|p| p.idl)?
                .ok_or_else(|| {
                    anyhow!(
                        "IDL not found. Please build the program atleast once to generate the IDL."
                    )
                })
        },
        |idl_path| {
            let idl = fs::read(idl_path)?;
            let idl = convert_idl(&idl)?;
            if idl.metadata.name != program_name {
                return Err(anyhow!("IDL does not match program {program_name}."));
            }

            Ok(idl)
        },
    )?;

    let cluster = match &cfg_override.cluster {
        Some(cluster) => cluster.clone(),
        None => Config::discover(cfg_override)?
            .map(|cfg| cfg.provider.cluster.clone())
            .unwrap_or(Cluster::Localnet),
    };

    let data = create_client(cluster.url()).get_account_data(&address)?;
    let disc_len = idl
        .accounts
        .iter()
        .find(|acc| acc.name == account_type_name)
        .map(|acc| acc.discriminator.len())
        .ok_or_else(|| anyhow!("Account `{account_type_name}` not found in IDL"))?;
    let mut data_view = &data[disc_len..];

    let deserialized_json =
        deserialize_idl_defined_type_to_json(&idl, account_type_name, &mut data_view)?;

    println!(
        "{}",
        serde_json::to_string_pretty(&deserialized_json).unwrap()
    );

    Ok(())
}

// Deserializes user defined IDL types by munching the account data(recursively).
fn deserialize_idl_defined_type_to_json(
    idl: &Idl,
    defined_type_name: &str,
    data: &mut &[u8],
) -> Result<JsonValue, anyhow::Error> {
    let defined_type = &idl
        .accounts
        .iter()
        .find(|acc| acc.name == defined_type_name)
        .and_then(|acc| idl.types.iter().find(|ty| ty.name == acc.name))
        .or_else(|| idl.types.iter().find(|ty| ty.name == defined_type_name))
        .ok_or_else(|| anyhow!("Type `{}` not found in IDL.", defined_type_name))?
        .ty;

    let mut deserialized_fields = Map::new();

    match defined_type {
        IdlTypeDefTy::Struct { fields } => {
            if let Some(fields) = fields {
                match fields {
                    IdlDefinedFields::Named(fields) => {
                        for field in fields {
                            deserialized_fields.insert(
                                field.name.clone(),
                                deserialize_idl_type_to_json(&field.ty, data, idl)?,
                            );
                        }
                    }
                    IdlDefinedFields::Tuple(fields) => {
                        let mut values = Vec::new();
                        for field in fields {
                            values.push(deserialize_idl_type_to_json(field, data, idl)?);
                        }
                        deserialized_fields
                            .insert(defined_type_name.to_owned(), JsonValue::Array(values));
                    }
                }
            }
        }
        IdlTypeDefTy::Enum { variants } => {
            let repr = <u8 as AnchorDeserialize>::deserialize(data)?;

            let variant = variants
                .get(repr as usize)
                .ok_or_else(|| anyhow!("Error while deserializing enum variant {repr}"))?;

            let mut value = json!({});

            if let Some(enum_field) = &variant.fields {
                match enum_field {
                    IdlDefinedFields::Named(fields) => {
                        let mut values = Map::new();
                        for field in fields {
                            values.insert(
                                field.name.clone(),
                                deserialize_idl_type_to_json(&field.ty, data, idl)?,
                            );
                        }
                        value = JsonValue::Object(values);
                    }
                    IdlDefinedFields::Tuple(fields) => {
                        let mut values = Vec::new();
                        for field in fields {
                            values.push(deserialize_idl_type_to_json(field, data, idl)?);
                        }
                        value = JsonValue::Array(values);
                    }
                }
            }

            deserialized_fields.insert(variant.name.clone(), value);
        }
        IdlTypeDefTy::Type { alias } => {
            return deserialize_idl_type_to_json(alias, data, idl);
        }
    }

    Ok(JsonValue::Object(deserialized_fields))
}

// Deserializes a primitive type using AnchorDeserialize
fn deserialize_idl_type_to_json(
    idl_type: &IdlType,
    data: &mut &[u8],
    parent_idl: &Idl,
) -> Result<JsonValue, anyhow::Error> {
    if data.is_empty() {
        return Err(anyhow::anyhow!("Unable to parse from empty bytes"));
    }

    Ok(match idl_type {
        IdlType::Bool => json!(<bool as AnchorDeserialize>::deserialize(data)?),
        IdlType::U8 => {
            json!(<u8 as AnchorDeserialize>::deserialize(data)?)
        }
        IdlType::I8 => {
            json!(<i8 as AnchorDeserialize>::deserialize(data)?)
        }
        IdlType::U16 => {
            json!(<u16 as AnchorDeserialize>::deserialize(data)?)
        }
        IdlType::I16 => {
            json!(<i16 as AnchorDeserialize>::deserialize(data)?)
        }
        IdlType::U32 => {
            json!(<u32 as AnchorDeserialize>::deserialize(data)?)
        }
        IdlType::I32 => {
            json!(<i32 as AnchorDeserialize>::deserialize(data)?)
        }
        IdlType::F32 => json!(<f32 as AnchorDeserialize>::deserialize(data)?),
        IdlType::U64 => {
            json!(<u64 as AnchorDeserialize>::deserialize(data)?)
        }
        IdlType::I64 => {
            json!(<i64 as AnchorDeserialize>::deserialize(data)?)
        }
        IdlType::F64 => json!(<f64 as AnchorDeserialize>::deserialize(data)?),
        IdlType::U128 => {
            json!(<u128 as AnchorDeserialize>::deserialize(data)?)
        }
        IdlType::I128 => {
            json!(<i128 as AnchorDeserialize>::deserialize(data)?)
        }
        IdlType::U256 => todo!("Upon completion of u256 IDL standard"),
        IdlType::I256 => todo!("Upon completion of i256 IDL standard"),
        IdlType::Bytes => JsonValue::Array(
            <Vec<u8> as AnchorDeserialize>::deserialize(data)?
                .iter()
                .map(|i| json!(*i))
                .collect(),
        ),
        IdlType::String => json!(<String as AnchorDeserialize>::deserialize(data)?),
        IdlType::Pubkey => {
            json!(<Pubkey as AnchorDeserialize>::deserialize(data)?.to_string())
        }
        IdlType::Array(ty, size) => match size {
            IdlArrayLen::Value(size) => {
                let mut array_data: Vec<JsonValue> = Vec::with_capacity(*size);

                for _ in 0..*size {
                    array_data.push(deserialize_idl_type_to_json(ty, data, parent_idl)?);
                }

                JsonValue::Array(array_data)
            }
            // TODO:
            IdlArrayLen::Generic(_) => unimplemented!("Generic array length is not yet supported"),
        },
        IdlType::Option(ty) => {
            let is_present = <u8 as AnchorDeserialize>::deserialize(data)?;

            if is_present == 0 {
                JsonValue::String("None".to_string())
            } else {
                deserialize_idl_type_to_json(ty, data, parent_idl)?
            }
        }
        IdlType::Vec(ty) => {
            let size: usize = <u32 as AnchorDeserialize>::deserialize(data)?
                .try_into()
                .unwrap();

            let mut vec_data: Vec<JsonValue> = Vec::with_capacity(size);

            for _ in 0..size {
                vec_data.push(deserialize_idl_type_to_json(ty, data, parent_idl)?);
            }

            JsonValue::Array(vec_data)
        }
        IdlType::Defined {
            name,
            generics: _generics,
        } => {
            // TODO: Generics
            deserialize_idl_defined_type_to_json(parent_idl, name, data)?
        }
        IdlType::Generic(generic) => json!(generic),
        _ => unimplemented!("{idl_type:?}"),
    })
}

enum OutFile {
    Stdout,
    File(PathBuf),
}

// Builds, deploys, and tests all workspace programs in a single command.
#[allow(clippy::too_many_arguments)]
fn test(
    cfg_override: &ConfigOverride,
    program_name: Option<String>,
    skip_deploy: bool,
    skip_local_validator: bool,
    skip_build: bool,
    skip_lint: bool,
    no_idl: bool,
    detach: bool,
    tests_to_run: Vec<String>,
    extra_args: Vec<String>,
    env_vars: Vec<String>,
    cargo_args: Vec<String>,
    arch: ProgramArch,
) -> Result<()> {
    let test_paths = tests_to_run
        .iter()
        .map(|path| {
            PathBuf::from(path)
                .canonicalize()
                .map_err(|_| anyhow!("Wrong path {}", path))
        })
        .collect::<Result<Vec<_>, _>>()?;

    with_workspace(cfg_override, |cfg| {
        // Build if needed.
        if !skip_build {
            build(
                cfg_override,
                no_idl,
                None,
                None,
                false,
                skip_lint,
                true,
                program_name.clone(),
                None,
                None,
                BootstrapMode::None,
                None,
                None,
                env_vars,
                cargo_args,
                false,
                arch,
            )?;
        }

        let root = cfg.path().parent().unwrap().to_owned();
        cfg.add_test_config(root, test_paths)?;

        // Run the deploy against the cluster in two cases:
        //
        // 1. The cluster is not localnet.
        // 2. The cluster is localnet, but we're not booting a local validator.
        //
        // In either case, skip the deploy if the user specifies.
        let is_localnet = cfg.provider.cluster == Cluster::Localnet;
        if (!is_localnet || skip_local_validator) && !skip_deploy {
            deploy(cfg_override, None, None, false, true, vec![])?;
        }

        cfg.run_hooks(HookType::PreTest)?;

        let mut is_first_suite = true;
        if let Some(test_script) = cfg.scripts.get_mut("test") {
            is_first_suite = false;

            match program_name {
                Some(program_name) => {
                    if let Some((from, to)) = Regex::new("\\s(tests/\\S+\\.(js|ts))")
                        .unwrap()
                        .captures_iter(&test_script.clone())
                        .last()
                        .and_then(|c| c.get(1).and_then(|mtch| c.get(2).map(|ext| (mtch, ext))))
                        .map(|(mtch, ext)| {
                            (
                                mtch.as_str(),
                                format!("tests/{program_name}.{}", ext.as_str()),
                            )
                        })
                    {
                        println!("\nRunning tests of program `{program_name}`!");
                        // Replace the last path to the program name's path
                        *test_script = test_script.replace(from, &to);
                    }
                }
                _ => println!(
                    "\nFound a 'test' script in the Anchor.toml. Running it as a test suite!"
                ),
            }

            run_test_suite(
                cfg,
                cfg.path(),
                is_localnet,
                skip_local_validator,
                skip_deploy,
                detach,
                &cfg.test_validator,
                &cfg.scripts,
                &extra_args,
            )?;
        }
        if let Some(test_config) = &cfg.test_config {
            for test_suite in test_config.iter() {
                if !is_first_suite {
                    std::thread::sleep(std::time::Duration::from_millis(
                        test_suite
                            .1
                            .test
                            .as_ref()
                            .map(|val| val.shutdown_wait)
                            .unwrap_or(SHUTDOWN_WAIT) as u64,
                    ));
                } else {
                    is_first_suite = false;
                }

                run_test_suite(
                    cfg,
                    test_suite.0,
                    is_localnet,
                    skip_local_validator,
                    skip_deploy,
                    detach,
                    &test_suite.1.test,
                    &test_suite.1.scripts,
                    &extra_args,
                )?;
            }
        }
        cfg.run_hooks(HookType::PostTest)?;
        Ok(())
    })
}

#[allow(clippy::too_many_arguments)]
fn run_test_suite(
    cfg: &WithPath<Config>,
    test_suite_path: impl AsRef<Path>,
    is_localnet: bool,
    skip_local_validator: bool,
    skip_deploy: bool,
    detach: bool,
    test_validator: &Option<TestValidator>,
    scripts: &ScriptsConfig,
    extra_args: &[String],
) -> Result<()> {
    println!("\nRunning test suite: {:#?}\n", test_suite_path.as_ref());
    // Start local test validator, if needed.
    let mut validator_handle = None;
    if is_localnet && (!skip_local_validator) {
        let flags = match skip_deploy {
            true => None,
            false => Some(validator_flags(cfg, test_validator)?),
        };
        validator_handle = Some(start_test_validator(cfg, test_validator, flags, true)?);
    }

    let url = cluster_url(cfg, test_validator);

    let node_options = format!(
        "{} {}",
        match std::env::var_os("NODE_OPTIONS") {
            Some(value) => value
                .into_string()
                .map_err(std::env::VarError::NotUnicode)?,
            None => "".to_owned(),
        },
        get_node_dns_option()?,
    );

    // Setup log reader.
    let log_streams = stream_logs(cfg, &url);

    // Run the tests.
    let test_result = {
        let cmd = scripts
            .get("test")
            .expect("Not able to find script for `test`")
            .clone();
        let script_args = format!("{cmd} {}", extra_args.join(" "));
        std::process::Command::new("bash")
            .arg("-c")
            .arg(script_args)
            .env("ANCHOR_PROVIDER_URL", url)
            .env("ANCHOR_WALLET", cfg.provider.wallet.to_string())
            .env("NODE_OPTIONS", node_options)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(anyhow::Error::from)
            .context(cmd)
    };

    // Keep validator running if needed.
    if test_result.is_ok() && detach {
        println!("Local validator still running. Press Ctrl + C quit.");
        std::io::stdin().lock().lines().next().unwrap().unwrap();
    }

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

    // Must exist *after* shutting down the validator and log streams.
    match test_result {
        Ok(exit) => {
            if !exit.status.success() {
                std::process::exit(exit.status.code().unwrap());
            }
        }
        Err(err) => {
            println!("Failed to run test: {err:#}");
            return Err(err);
        }
    }

    Ok(())
}

// Returns the solana-test-validator flags. This will embed the workspace
// programs in the genesis block so we don't have to deploy every time. It also
// allows control of other solana-test-validator features.
fn validator_flags(
    cfg: &WithPath<Config>,
    test_validator: &Option<TestValidator>,
) -> Result<Vec<String>> {
    let programs = cfg.programs.get(&Cluster::Localnet);

    let test_upgradeable_program = test_validator
        .as_ref()
        .map(|test_validator| test_validator.upgradeable)
        .unwrap_or(false);

    let mut flags = Vec::new();
    for mut program in cfg.read_all_programs()? {
        let verifiable = false;
        let binary_path = program.binary_path(verifiable).display().to_string();

        // Use the [programs.cluster] override and fallback to the keypair
        // files if no override is given.
        let address = programs
            .and_then(|m| m.get(&program.lib_name))
            .map(|deployment| Ok(deployment.address.to_string()))
            .unwrap_or_else(|| program.pubkey().map(|p| p.to_string()))?;

        if test_upgradeable_program {
            flags.push("--upgradeable-program".to_string());
            flags.push(address.clone());
            flags.push(binary_path);
            flags.push(cfg.wallet_kp()?.pubkey().to_string());
        } else {
            flags.push("--bpf-program".to_string());
            flags.push(address.clone());
            flags.push(binary_path);
        }

        if let Some(idl) = program.idl.as_mut() {
            // Add program address to the IDL.
            idl.address = address;

            // Persist it.
            let idl_out = Path::new("target")
                .join("idl")
                .join(&idl.metadata.name)
                .with_extension("json");
            write_idl(idl, OutFile::File(idl_out))?;
        }
    }

    if let Some(test) = test_validator.as_ref() {
        if let Some(genesis) = &test.genesis {
            for entry in genesis {
                let program_path = Path::new(&entry.program);
                if !program_path.exists() {
                    return Err(anyhow!(
                        "Program in genesis configuration does not exist at path: {}",
                        program_path.display()
                    ));
                }
                if entry.upgradeable.unwrap_or(false) {
                    flags.push("--upgradeable-program".to_string());
                    flags.push(entry.address.clone());
                    flags.push(entry.program.clone());
                    flags.push(cfg.wallet_kp()?.pubkey().to_string());
                } else {
                    flags.push("--bpf-program".to_string());
                    flags.push(entry.address.clone());
                    flags.push(entry.program.clone());
                }
            }
        }
        if let Some(validator) = &test.validator {
            let entries = serde_json::to_value(validator)?;
            for (key, value) in entries.as_object().unwrap() {
                if key == "ledger" {
                    // Ledger flag is a special case as it is passed separately to the rest of
                    // these validator flags.
                    continue;
                };
                if key == "account" {
                    for entry in value.as_array().unwrap() {
                        // Push the account flag for each array entry
                        flags.push("--account".to_string());
                        flags.push(entry["address"].as_str().unwrap().to_string());
                        flags.push(entry["filename"].as_str().unwrap().to_string());
                    }
                } else if key == "account_dir" {
                    for entry in value.as_array().unwrap() {
                        flags.push("--account-dir".to_string());
                        flags.push(entry["directory"].as_str().unwrap().to_string());
                    }
                } else if key == "clone" {
                    // Client for fetching accounts data
                    let client = if let Some(url) = entries["url"].as_str() {
                        create_client(url)
                    } else {
                        return Err(anyhow!(
                            "Validator url for Solana's JSON RPC should be provided in order to clone accounts from it"
                        ));
                    };

                    let pubkeys = value
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|entry| {
                            let address = entry["address"].as_str().unwrap();
                            Pubkey::try_from(address)
                                .map_err(|_| anyhow!("Invalid pubkey {}", address))
                        })
                        .collect::<Result<HashSet<Pubkey>>>()?
                        .into_iter()
                        .collect::<Vec<_>>();
                    let accounts = client.get_multiple_accounts(&pubkeys)?;

                    for (pubkey, account) in pubkeys.into_iter().zip(accounts) {
                        match account {
                            Some(account) => {
                                // Use a different flag for program accounts to fix the problem
                                // described in https://github.com/anza-xyz/agave/issues/522
                                if account.owner == bpf_loader_upgradeable::id()
                                    // Only programs are supported with `--clone-upgradeable-program`
                                    && matches!(
                                        account.deserialize_data::<UpgradeableLoaderState>()?,
                                        UpgradeableLoaderState::Program { .. }
                                    )
                                {
                                    flags.push("--clone-upgradeable-program".to_string());
                                    flags.push(pubkey.to_string());
                                } else {
                                    flags.push("--clone".to_string());
                                    flags.push(pubkey.to_string());
                                }
                            }
                            _ => return Err(anyhow!("Account {} not found", pubkey)),
                        }
                    }
                } else if key == "deactivate_feature" {
                    // Verify that the feature flags are valid pubkeys
                    let pubkeys_result: Result<Vec<Pubkey>, _> = value
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|entry| {
                            let feature_flag = entry.as_str().unwrap();
                            Pubkey::try_from(feature_flag).map_err(|_| {
                                anyhow!("Invalid pubkey (feature flag) {}", feature_flag)
                            })
                        })
                        .collect();
                    let features = pubkeys_result?;
                    for feature in features {
                        flags.push("--deactivate-feature".to_string());
                        flags.push(feature.to_string());
                    }
                } else {
                    // Remaining validator flags are non-array types
                    flags.push(format!("--{}", key.replace('_', "-")));
                    if let serde_json::Value::String(v) = value {
                        flags.push(v.to_string());
                    } else {
                        flags.push(value.to_string());
                    }
                }
            }
        }
    }

    Ok(flags)
}

fn stream_logs(config: &WithPath<Config>, rpc_url: &str) -> Result<Vec<std::process::Child>> {
    let program_logs_dir = Path::new(".anchor").join("program-logs");
    if program_logs_dir.exists() {
        fs::remove_dir_all(&program_logs_dir)
            .with_context(|| format!("Failed to remove dir {}", program_logs_dir.display()))?;
    }
    fs::create_dir_all(&program_logs_dir)
        .with_context(|| format!("Failed to create dir {}", program_logs_dir.display()))?;

    let mut handles = vec![];
    for program in config.read_all_programs()? {
        let idl_path = Path::new("target")
            .join("idl")
            .join(&program.lib_name)
            .with_extension("json");
        let idl = fs::read(&idl_path)
            .with_context(|| format!("Failed to read IDL file {}", idl_path.display()))?;
        let idl = convert_idl(&idl)?;

        let log_path = program_logs_dir.join(format!("{}.{}.log", &idl.address, program.lib_name));
        let log_file = File::create(&log_path)
            .with_context(|| format!("Failed to create log file {}", log_path.display()))?;
        let stdio = std::process::Stdio::from(log_file);
        let child = std::process::Command::new("solana")
            .arg("logs")
            .arg(&idl.address)
            .arg("--url")
            .arg(rpc_url)
            .stdout(stdio)
            .spawn()
            .with_context(|| format!("Failed to spawn 'solana logs' for {}", &idl.address))?;
        handles.push(child);
    }
    if let Some(test) = config.test_validator.as_ref() {
        if let Some(genesis) = &test.genesis {
            for entry in genesis {
                let genesis_log_path = program_logs_dir.join(&entry.address).with_extension("log");
                let log_file = File::create(&genesis_log_path).with_context(|| {
                    format!("Failed to create log file {}", genesis_log_path.display())
                })?;
                let stdio = std::process::Stdio::from(log_file);
                let child = std::process::Command::new("solana")
                    .arg("logs")
                    .arg(entry.address.clone())
                    .arg("--url")
                    .arg(rpc_url)
                    .stdout(stdio)
                    .spawn()
                    .with_context(|| {
                        "Failed to spawn 'solana logs' for genesis entry".to_string()
                    })?;
                handles.push(child);
            }
        }
    }

    Ok(handles)
}

fn start_test_validator(
    cfg: &Config,
    test_validator: &Option<TestValidator>,
    flags: Option<Vec<String>>,
    test_log_stdout: bool,
) -> Result<Child> {
    let (test_ledger_directory, test_ledger_log_filename) =
        test_validator_file_paths(test_validator)?;

    // Start a validator for testing.
    let (test_validator_stdout, test_validator_stderr) = match test_log_stdout {
        true => {
            let test_validator_stdout_file =
                File::create(&test_ledger_log_filename).with_context(|| {
                    format!(
                        "Failed to create validator log file {}",
                        test_ledger_log_filename.display()
                    )
                })?;
            let test_validator_sterr_file = test_validator_stdout_file.try_clone()?;
            (
                Stdio::from(test_validator_stdout_file),
                Stdio::from(test_validator_sterr_file),
            )
        }
        false => (Stdio::inherit(), Stdio::inherit()),
    };

    let rpc_url = test_validator_rpc_url(test_validator);

    let rpc_port = cfg
        .test_validator
        .as_ref()
        .and_then(|test| test.validator.as_ref().map(|v| v.rpc_port))
        .unwrap_or(DEFAULT_RPC_PORT);
    if !portpicker::is_free(rpc_port) {
        return Err(anyhow!(
            "Your configured rpc port: {rpc_port} is already in use"
        ));
    }
    let faucet_port = cfg
        .test_validator
        .as_ref()
        .and_then(|test| test.validator.as_ref().and_then(|v| v.faucet_port))
        .unwrap_or(solana_faucet::faucet::FAUCET_PORT);
    if !portpicker::is_free(faucet_port) {
        return Err(anyhow!(
            "Your configured faucet port: {faucet_port} is already in use"
        ));
    }

    let mut validator_handle = std::process::Command::new("solana-test-validator")
        .arg("--ledger")
        .arg(test_ledger_directory)
        .arg("--mint")
        .arg(cfg.wallet_kp()?.pubkey().to_string())
        .args(flags.unwrap_or_default())
        .stdout(test_validator_stdout)
        .stderr(test_validator_stderr)
        .spawn()
        .map_err(|e| anyhow!("Failed to spawn `solana-test-validator`: {e}"))?;

    // Wait for the validator to be ready.
    let client = create_client(rpc_url);
    let mut count = 0;
    let ms_wait = test_validator
        .as_ref()
        .map(|test| test.startup_wait)
        .unwrap_or(STARTUP_WAIT);
    while count < ms_wait {
        let r = client.get_latest_blockhash();
        if r.is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        count += 100;
    }
    if count >= ms_wait {
        eprintln!(
            "Unable to get latest blockhash. Test validator does not look started. \
            Check {test_ledger_log_filename:?} for errors. Consider increasing [test.startup_wait] in Anchor.toml."
        );
        validator_handle.kill()?;
        std::process::exit(1);
    }
    Ok(validator_handle)
}

// Return the URL that solana-test-validator should be running on given the
// configuration
fn test_validator_rpc_url(test_validator: &Option<TestValidator>) -> String {
    match test_validator {
        Some(TestValidator {
            validator: Some(validator),
            ..
        }) => format!("http://{}:{}", validator.bind_address, validator.rpc_port),
        _ => "http://127.0.0.1:8899".to_string(),
    }
}

// Setup and return paths to the solana-test-validator ledger directory and log
// files given the configuration
fn test_validator_file_paths(test_validator: &Option<TestValidator>) -> Result<(PathBuf, PathBuf)> {
    let ledger_path = match test_validator {
        Some(TestValidator {
            validator: Some(validator),
            ..
        }) => PathBuf::from(&validator.ledger),
        _ => get_default_ledger_path(),
    };

    if !ledger_path.is_relative() {
        // Prevent absolute paths to avoid someone using / or similar, as the
        // directory gets removed
        eprintln!("Ledger directory {ledger_path:?} must be relative");
        std::process::exit(1);
    }
    if ledger_path.exists() {
        fs::remove_dir_all(&ledger_path).with_context(|| {
            format!(
                "Failed to remove ledger directory {}",
                ledger_path.display()
            )
        })?;
    }

    fs::create_dir_all(&ledger_path).with_context(|| {
        format!(
            "Failed to create ledger directory {}",
            ledger_path.display()
        )
    })?;

    let log_path = ledger_path.join("test-ledger-log.txt");
    Ok((ledger_path, log_path))
}

fn cluster_url(cfg: &Config, test_validator: &Option<TestValidator>) -> String {
    let is_localnet = cfg.provider.cluster == Cluster::Localnet;
    match is_localnet {
        // Cluster is Localnet, assume the intent is to use the configuration
        // for solana-test-validator
        true => test_validator_rpc_url(test_validator),
        false => cfg.provider.cluster.url().to_string(),
    }
}

fn clean(cfg_override: &ConfigOverride) -> Result<()> {
    let cfg = Config::discover(cfg_override)?.expect("Not in workspace.");
    let cfg_parent = cfg.path().parent().expect("Invalid Anchor.toml");
    let dot_anchor_dir = cfg_parent.join(".anchor");
    let target_dir = cfg_parent.join("target");
    let deploy_dir = target_dir.join("deploy");

    if dot_anchor_dir.exists() {
        fs::remove_dir_all(&dot_anchor_dir)
            .map_err(|e| anyhow!("Could not remove directory {:?}: {}", dot_anchor_dir, e))?;
    }

    if target_dir.exists() {
        for entry in fs::read_dir(target_dir)? {
            let path = entry?.path();
            if path.is_dir() && path != deploy_dir {
                fs::remove_dir_all(&path)
                    .map_err(|e| anyhow!("Could not remove directory {}: {}", path.display(), e))?;
            } else if path.is_file() {
                fs::remove_file(&path)
                    .map_err(|e| anyhow!("Could not remove file {}: {}", path.display(), e))?;
            }
        }
    } else {
        println!("skipping target directory: not found")
    }

    if deploy_dir.exists() {
        for file in fs::read_dir(deploy_dir)? {
            let path = file?.path();
            if path.extension() != Some(&OsString::from("json")) {
                fs::remove_file(&path)
                    .map_err(|e| anyhow!("Could not remove file {}: {}", path.display(), e))?;
            }
        }
    } else {
        println!("skipping deploy directory: not found")
    }

    Ok(())
}

fn deploy(
    cfg_override: &ConfigOverride,
    program_name: Option<String>,
    program_keypair: Option<String>,
    verifiable: bool,
    no_idl: bool,
    solana_args: Vec<String>,
) -> Result<()> {
    // Execute the code within the workspace
    with_workspace(cfg_override, |cfg| {
        let url = cluster_url(cfg, &cfg.test_validator);
        let keypair = cfg.provider.wallet.to_string();

        // Augment the given solana args with recommended defaults.
        let client = create_client(&url);
        let solana_args = add_recommended_deployment_solana_args(&client, solana_args)?;

        cfg.run_hooks(HookType::PreDeploy)?;
        // Deploy the programs.
        println!("Deploying cluster: {url}");
        println!("Upgrade authority: {keypair}");

        for mut program in cfg.get_programs(program_name)? {
            let binary_path = program.binary_path(verifiable).display().to_string();

            println!("Deploying program {:?}...", program.lib_name);
            println!("Program path: {binary_path}...");

            let (program_keypair_filepath, program_id) = match &program_keypair {
                Some(path) => (path.clone(), get_keypair(path)?.pubkey()),
                None => (
                    program.keypair_file()?.path().display().to_string(),
                    program.pubkey()?,
                ),
            };

            // Send deploy transactions using the Solana CLI
            let exit = std::process::Command::new("solana")
                .arg("program")
                .arg("deploy")
                .arg("--url")
                .arg(&url)
                .arg("--keypair")
                .arg(&keypair)
                .arg("--program-id")
                .arg(strip_workspace_prefix(program_keypair_filepath))
                .arg(strip_workspace_prefix(binary_path))
                .args(&solana_args)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()
                .expect("Must deploy");

            // Check if deployment was successful
            if !exit.status.success() {
                println!("There was a problem deploying: {exit:?}.");
                std::process::exit(exit.status.code().unwrap_or(1));
            }

            // Get the IDL filepath
            let idl_filepath = Path::new("target")
                .join("idl")
                .join(&program.lib_name)
                .with_extension("json");

            if let Some(idl) = program.idl.as_mut() {
                // Add program address to the IDL.
                idl.address = program_id.to_string();

                // Persist it.
                write_idl(idl, OutFile::File(idl_filepath.clone()))?;

                // Upload the IDL to the cluster by default (unless no_idl is set)
                if !no_idl {
                    // Wait for the program to be confirmed before initializing IDL to prevent
                    // race condition where the program isn't yet available in validator cache
                    let client = create_client(&url);
                    let max_retries = 5;
                    let retry_delay = std::time::Duration::from_millis(500);
                    let cache_delay = std::time::Duration::from_secs(2);

                    println!("Waiting for program {program_id} to be confirmed...");

                    for attempt in 0..max_retries {
                        if let Ok(account) = client.get_account(&program_id) {
                            if account.executable {
                                println!("Program confirmed on-chain");
                                std::thread::sleep(cache_delay);
                                break;
                            }
                        }

                        if attempt == max_retries - 1 {
                            return Err(anyhow!(
                                "Timeout waiting for program {} to be confirmed",
                                program_id
                            ));
                        }

                        std::thread::sleep(retry_delay);
                    }

                    // Check if IDL account already exists
                    let (base, _) = Pubkey::find_program_address(&[], &program_id);
                    let idl_address = Pubkey::create_with_seed(&base, "anchor:idl", &program_id)
                        .expect("Seed is always valid");
                    let idl_account_exists = client.get_account(&idl_address).is_ok();

                    if idl_account_exists {
                        // IDL account exists, upgrade it
                        idl_upgrade(
                            cfg_override,
                            program_id,
                            idl_filepath.display().to_string(),
                            None,
                        )?;
                    } else {
                        // IDL account doesn't exist, create it
                        idl_init(
                            cfg_override,
                            program_id,
                            idl_filepath.display().to_string(),
                            None,
                            false,
                        )?;
                    }
                }
            }
        }

        println!("Deploy success");
        cfg.run_hooks(HookType::PostDeploy)?;

        Ok(())
    })
}

fn upgrade(
    cfg_override: &ConfigOverride,
    program_id: Pubkey,
    program_filepath: String,
    max_retries: u32,
    solana_args: Vec<String>,
) -> Result<()> {
    let path: PathBuf = program_filepath.parse().unwrap();
    let program_filepath = path.canonicalize()?.display().to_string();

    with_workspace(cfg_override, |cfg| {
        let url = cluster_url(cfg, &cfg.test_validator);
        let client = create_client(&url);
        let solana_args = add_recommended_deployment_solana_args(&client, solana_args)?;

        for retry in 0..(1 + max_retries) {
            let exit = std::process::Command::new("solana")
                .arg("program")
                .arg("deploy")
                .arg("--url")
                .arg(url.clone())
                .arg("--keypair")
                .arg(cfg.provider.wallet.to_string())
                .arg("--program-id")
                .arg(program_id.to_string())
                .arg(strip_workspace_prefix(program_filepath.clone()))
                .args(&solana_args)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()
                .expect("Must deploy");
            if exit.status.success() {
                break;
            }

            println!("There was a problem deploying: {exit:?}.");
            if retry < max_retries {
                println!("Retrying {} more time(s)...", max_retries - retry);
            } else {
                std::process::exit(exit.status.code().unwrap_or(1));
            }
        }
        Ok(())
    })
}

fn migrate(cfg_override: &ConfigOverride) -> Result<()> {
    with_workspace(cfg_override, |cfg| {
        println!("Running migration deploy script");

        let url = cluster_url(cfg, &cfg.test_validator);
        let cur_dir = std::env::current_dir()?;
        let migrations_dir = cur_dir.join("migrations");
        let deploy_ts = Path::new("deploy.ts");

        let use_ts = Path::new("tsconfig.json").exists() && migrations_dir.join(deploy_ts).exists();

        if !Path::new(".anchor").exists() {
            fs::create_dir(".anchor")?;
        }
        std::env::set_current_dir(".anchor")?;

        let exit = if use_ts {
            let module_path = migrations_dir.join(deploy_ts);
            let deploy_script_host_str =
                rust_template::deploy_ts_script_host(&url, &module_path.display().to_string());
            fs::write(deploy_ts, deploy_script_host_str)?;

            let pkg_manager_cmd = match &cfg.toolchain.package_manager {
                Some(pkg_manager) => pkg_manager.to_string(),
                None => PackageManager::default().to_string(),
            };

            std::process::Command::new(pkg_manager_cmd)
                .args([
                    "run",
                    "ts-node",
                    &fs::canonicalize(deploy_ts)?.to_string_lossy(),
                ])
                .env("ANCHOR_WALLET", cfg.provider.wallet.to_string())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()?
        } else {
            let deploy_js = deploy_ts.with_extension("js");
            let module_path = migrations_dir.join(&deploy_js);
            let deploy_script_host_str =
                rust_template::deploy_js_script_host(&url, &module_path.display().to_string());
            fs::write(&deploy_js, deploy_script_host_str)?;

            std::process::Command::new("node")
                .arg(&deploy_js)
                .env("ANCHOR_WALLET", cfg.provider.wallet.to_string())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()?
        };

        if !exit.status.success() {
            eprintln!("Deploy failed.");
            std::process::exit(exit.status.code().unwrap());
        }

        println!("Deploy complete.");
        Ok(())
    })
}

fn set_workspace_dir_or_exit() {
    let d = match Config::discover(&ConfigOverride::default()) {
        Err(err) => {
            println!("Workspace configuration error: {err}");
            std::process::exit(1);
        }
        Ok(d) => d,
    };
    match d {
        None => {
            println!("Not in anchor workspace.");
            std::process::exit(1);
        }
        Some(cfg) => {
            match cfg.path().parent() {
                None => {
                    println!("Unable to make new program");
                }
                Some(parent) => {
                    if std::env::set_current_dir(parent).is_err() {
                        println!("Not in anchor workspace.");
                        std::process::exit(1);
                    }
                }
            };
        }
    }
}

#[cfg(feature = "dev")]
fn airdrop(cfg_override: &ConfigOverride) -> Result<()> {
    let url = cfg_override
        .cluster
        .as_ref()
        .unwrap_or(&Cluster::Devnet)
        .url();
    loop {
        let exit = std::process::Command::new("solana")
            .arg("airdrop")
            .arg("10")
            .arg("--url")
            .arg(url)
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
    println!("* Mainnet - https://api.mainnet-beta.solana.com");
    println!("* Devnet  - https://api.devnet.solana.com");
    println!("* Testnet - https://api.testnet.solana.com");
    Ok(())
}

fn shell(cfg_override: &ConfigOverride) -> Result<()> {
    with_workspace(cfg_override, |cfg| {
        let programs = {
            // Create idl map from all workspace programs.
            let mut idls: HashMap<String, Idl> = cfg
                .read_all_programs()?
                .iter()
                .filter(|program| program.idl.is_some())
                .map(|program| {
                    (
                        program.idl.as_ref().unwrap().metadata.name.clone(),
                        program.idl.clone().unwrap(),
                    )
                })
                .collect();
            // Insert all manually specified idls into the idl map.
            if let Some(programs) = cfg.programs.get(&cfg.provider.cluster) {
                let _ = programs
                    .iter()
                    .map(|(name, pd)| {
                        if let Some(idl_fp) = &pd.idl {
                            let file_str =
                                fs::read_to_string(idl_fp).expect("Unable to read IDL file");
                            let idl = serde_json::from_str(&file_str).expect("Idl not readable");
                            idls.insert(name.clone(), idl);
                        }
                    })
                    .collect::<Vec<_>>();
            }

            // Finalize program list with all programs with IDLs.
            match cfg.programs.get(&cfg.provider.cluster) {
                None => Vec::new(),
                Some(programs) => programs
                    .iter()
                    .filter_map(|(name, program_deployment)| {
                        Some(ProgramWorkspace {
                            name: name.to_string(),
                            program_id: program_deployment.address,
                            idl: match idls.get(name) {
                                None => return None,
                                Some(idl) => idl.clone(),
                            },
                        })
                    })
                    .collect::<Vec<ProgramWorkspace>>(),
            }
        };
        let url = cluster_url(cfg, &cfg.test_validator);
        let js_code = rust_template::node_shell(&url, &cfg.provider.wallet.to_string(), programs)?;
        let mut child = std::process::Command::new("node")
            .args(["-e", &js_code, "-i", "--experimental-repl-await"])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| anyhow::format_err!("{}", e))?;

        if !child.wait()?.success() {
            println!("Error running node shell");
            return Ok(());
        }
        Ok(())
    })
}

fn run(cfg_override: &ConfigOverride, script: String, script_args: Vec<String>) -> Result<()> {
    with_workspace(cfg_override, |cfg| {
        let url = cluster_url(cfg, &cfg.test_validator);
        let script = cfg
            .scripts
            .get(&script)
            .ok_or_else(|| anyhow!("Unable to find script"))?;
        let script_with_args = format!("{script} {}", script_args.join(" "));
        let exit = std::process::Command::new("bash")
            .arg("-c")
            .arg(&script_with_args)
            .env("ANCHOR_PROVIDER_URL", url)
            .env("ANCHOR_WALLET", cfg.provider.wallet.to_string())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .unwrap();
        if !exit.status.success() {
            std::process::exit(exit.status.code().unwrap_or(1));
        }
        Ok(())
    })
}

fn login(_cfg_override: &ConfigOverride, token: String) -> Result<()> {
    let anchor_dir = Path::new(&*shellexpand::tilde("~"))
        .join(".config")
        .join("anchor");
    if !anchor_dir.exists() {
        fs::create_dir(&anchor_dir)?;
    }

    std::env::set_current_dir(&anchor_dir)?;

    // Freely overwrite the entire file since it's not used for anything else.
    let mut file = File::create("credentials")?;
    file.write_all(rust_template::credentials(&token).as_bytes())?;
    Ok(())
}

fn keys(cfg_override: &ConfigOverride, cmd: KeysCommand) -> Result<()> {
    match cmd {
        KeysCommand::List => keys_list(cfg_override),
        KeysCommand::Sync { program_name } => keys_sync(cfg_override, program_name),
    }
}

fn keys_list(cfg_override: &ConfigOverride) -> Result<()> {
    with_workspace(cfg_override, |cfg| {
        for program in cfg.read_all_programs()? {
            let pubkey = program.pubkey()?;
            println!("{}: {}", program.lib_name, pubkey);
        }
        Ok(())
    })
}

/// Sync program `declare_id!` pubkeys with the pubkey from `target/deploy/<KEYPAIR>.json`.
fn keys_sync(cfg_override: &ConfigOverride, program_name: Option<String>) -> Result<()> {
    with_workspace(cfg_override, |cfg| {
        let declare_id_regex = RegexBuilder::new(r#"^(([\w]+::)*)declare_id!\("(\w*)"\)"#)
            .multi_line(true)
            .build()
            .unwrap();

        let cfg_cluster = cfg.provider.cluster.to_owned();
        println!("Syncing program ids for the configured cluster ({cfg_cluster})\n");

        let mut changed_src = false;
        for program in cfg.get_programs(program_name)? {
            // Get the pubkey from the keypair file
            let actual_program_id = program.pubkey()?.to_string();

            // Handle declaration in program files
            let src_path = program.path.join("src");
            let files_to_check = vec![src_path.join("lib.rs"), src_path.join("id.rs")];

            for path in files_to_check {
                let mut content = match fs::read_to_string(&path) {
                    Ok(content) => content,
                    Err(_) => continue,
                };

                let incorrect_program_id = declare_id_regex
                    .captures(&content)
                    .and_then(|captures| captures.get(3))
                    .filter(|program_id_match| program_id_match.as_str() != actual_program_id);
                if let Some(program_id_match) = incorrect_program_id {
                    println!("Found incorrect program id declaration in {path:?}");

                    // Update the program id
                    content.replace_range(program_id_match.range(), &actual_program_id);
                    fs::write(&path, content)?;

                    changed_src = true;
                    println!("Updated to {actual_program_id}\n");
                    break;
                }
            }

            // Handle declaration in Anchor.toml
            'outer: for (cluster, programs) in &mut cfg.programs {
                // Only change if the configured cluster matches the program's cluster
                if cluster != &cfg_cluster {
                    continue;
                }

                for (name, deployment) in programs {
                    // Skip other programs
                    if name != &program.lib_name {
                        continue;
                    }

                    if deployment.address.to_string() != actual_program_id {
                        println!("Found incorrect program id declaration in Anchor.toml for the program `{name}`");

                        // Update the program id
                        deployment.address = Pubkey::try_from(actual_program_id.as_str()).unwrap();
                        fs::write(cfg.path(), cfg.to_string())?;

                        println!("Updated to {actual_program_id}\n");
                        break 'outer;
                    }
                }
            }
        }

        println!("All program id declarations are synced.");
        if changed_src {
            println!("Please rebuild the program to update the generated artifacts.")
        }

        Ok(())
    })
}

/// Check if there's a mismatch between the program keypair and the `declare_id!` in the source code.
/// Returns an error if a mismatch is detected, prompting the user to run `anchor keys sync`.
fn check_program_id_mismatch(cfg: &WithPath<Config>, program_name: Option<String>) -> Result<()> {
    let declare_id_regex = RegexBuilder::new(r#"^(([\w]+::)*)declare_id!\("(\w*)"\)"#)
        .multi_line(true)
        .build()
        .unwrap();

    for program in cfg.get_programs(program_name)? {
        // Get the pubkey from the keypair file
        let actual_program_id = program.pubkey()?.to_string();

        // Check declaration in program files
        let src_path = program.path.join("src");
        let files_to_check = vec![src_path.join("lib.rs"), src_path.join("id.rs")];

        for path in files_to_check {
            let content = match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(_) => continue,
            };

            let incorrect_program_id = declare_id_regex
                .captures(&content)
                .and_then(|captures| captures.get(3))
                .filter(|program_id_match| program_id_match.as_str() != actual_program_id);

            if let Some(program_id_match) = incorrect_program_id {
                let declared_id = program_id_match.as_str();
                return Err(anyhow!(
                    "Program ID mismatch detected for program '{}':\n  \
                    Keypair file has: {}\n  \
                    Source code has:  {}\n\n\
                    Please run 'anchor keys sync' to update the program ID in your source code or use the '--ignore-keys' flag to skip this check.",
                    program.lib_name,
                    actual_program_id,
                    declared_id
                ));
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn localnet(
    cfg_override: &ConfigOverride,
    skip_build: bool,
    skip_deploy: bool,
    skip_lint: bool,
    ignore_keys: bool,
    env_vars: Vec<String>,
    cargo_args: Vec<String>,
    arch: ProgramArch,
) -> Result<()> {
    with_workspace(cfg_override, |cfg| {
        // Build if needed.
        if !skip_build {
            build(
                cfg_override,
                false,
                None,
                None,
                false,
                skip_lint,
                ignore_keys,
                None,
                None,
                None,
                BootstrapMode::None,
                None,
                None,
                env_vars,
                cargo_args,
                false,
                arch,
            )?;
        }

        let flags = match skip_deploy {
            true => None,
            false => Some(validator_flags(cfg, &cfg.test_validator)?),
        };

        let validator_handle = &mut start_test_validator(cfg, &cfg.test_validator, flags, false)?;

        // Setup log reader.
        let url = test_validator_rpc_url(&cfg.test_validator);
        let log_streams = stream_logs(cfg, &url);

        std::io::stdin().lock().lines().next().unwrap().unwrap();

        // Check all errors and shut down.
        if let Err(err) = validator_handle.kill() {
            println!(
                "Failed to kill subprocess {}: {}",
                validator_handle.id(),
                err
            );
        }

        for mut child in log_streams? {
            if let Err(err) = child.kill() {
                println!("Failed to kill subprocess {}: {}", child.id(), err);
            }
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
fn with_workspace<R>(
    cfg_override: &ConfigOverride,
    f: impl FnOnce(&mut WithPath<Config>) -> R,
) -> R {
    set_workspace_dir_or_exit();

    let mut cfg = Config::discover(cfg_override)
        .expect("Previously set the workspace dir")
        .expect("Anchor.toml must always exist");

    let r = f(&mut cfg);

    set_workspace_dir_or_exit();

    r
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s == "." || s.starts_with('.') || s == "target")
        .unwrap_or(false)
}

fn get_node_version() -> Result<Version> {
    let node_version = std::process::Command::new("node")
        .arg("--version")
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| anyhow::format_err!("node failed: {}", e))?;
    let output = std::str::from_utf8(&node_version.stdout)?
        .strip_prefix('v')
        .unwrap()
        .trim();
    Version::parse(output).map_err(Into::into)
}

fn add_recommended_deployment_solana_args(
    client: &RpcClient,
    args: Vec<String>,
) -> Result<Vec<String>> {
    let mut augmented_args = args.clone();

    // If no priority fee is provided, calculate a recommended fee based on recent txs.
    if !args.contains(&"--with-compute-unit-price".to_string()) {
        let priority_fee = get_recommended_micro_lamport_fee(client)?;
        augmented_args.push("--with-compute-unit-price".to_string());
        augmented_args.push(priority_fee.to_string());
    }

    const DEFAULT_MAX_SIGN_ATTEMPTS: u8 = 30;
    if !args.contains(&"--max-sign-attempts".to_string()) {
        augmented_args.push("--max-sign-attempts".to_string());
        augmented_args.push(DEFAULT_MAX_SIGN_ATTEMPTS.to_string());
    }

    // If no buffer keypair is provided, create a temporary one to reuse across deployments.
    // This is particularly useful for upgrading larger programs, which suffer from an increased
    // likelihood of some write transactions failing during any single deployment.
    if !args.contains(&"--buffer".to_owned()) {
        let tmp_keypair_path = std::env::temp_dir().join("anchor-upgrade-buffer.json");
        if !tmp_keypair_path.exists() {
            if let Err(err) = Keypair::new().write_to_file(&tmp_keypair_path) {
                return Err(anyhow!(
                    "Error creating keypair for buffer account, {:?}",
                    err
                ));
            }
        }

        augmented_args.push("--buffer".to_owned());
        augmented_args.push(tmp_keypair_path.to_string_lossy().to_string());
    }

    Ok(augmented_args)
}

fn get_recommended_micro_lamport_fee(client: &RpcClient) -> Result<u64> {
    let mut fees = client.get_recent_prioritization_fees(&[])?;
    if fees.is_empty() {
        // Fees may be empty, e.g. on localnet
        return Ok(0);
    }

    // Get the median fee from the most recent 150 slots' prioritization fee
    fees.sort_unstable_by_key(|fee| fee.prioritization_fee);
    let median_index = fees.len() / 2;

    let median_priority_fee = if fees.len() % 2 == 0 {
        (fees[median_index - 1].prioritization_fee + fees[median_index].prioritization_fee) / 2
    } else {
        fees[median_index].prioritization_fee
    };

    Ok(median_priority_fee)
}

fn get_node_dns_option() -> Result<&'static str> {
    let version = get_node_version()?;
    let req = VersionReq::parse(">=16.4.0").unwrap();
    let option = match req.matches(&version) {
        true => "--dns-result-order=ipv4first",
        false => "",
    };
    Ok(option)
}

// Remove the current workspace directory if it prefixes a string.
// This is used as a workaround for the Solana CLI using the uriparse crate to
// parse args but not handling percent encoding/decoding when using the path as
// a local filesystem path. Removing the workspace prefix handles most/all cases
// of spaces in keypair/binary paths, but this should be fixed in the Solana CLI
// and removed here.
fn strip_workspace_prefix(absolute_path: String) -> String {
    let workspace_prefix =
        std::env::current_dir().unwrap().display().to_string() + std::path::MAIN_SEPARATOR_STR;
    absolute_path
        .strip_prefix(&workspace_prefix)
        .unwrap_or(&absolute_path)
        .into()
}

/// Create a new [`RpcClient`] with `confirmed` commitment level instead of the default(finalized).
fn create_client<U: ToString>(url: U) -> RpcClient {
    RpcClient::new_with_commitment(url, CommitmentConfig::confirmed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "Anchor workspace name must be a valid Rust identifier.")]
    fn test_init_reserved_word() {
        init(
            &ConfigOverride {
                cluster: None,
                wallet: None,
            },
            "await".to_string(),
            true,
            true,
            PackageManager::default(),
            false,
            ProgramTemplate::default(),
            TestTemplate::default(),
            false,
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Anchor workspace name must be a valid Rust identifier.")]
    fn test_init_reserved_word_from_syn() {
        init(
            &ConfigOverride {
                cluster: None,
                wallet: None,
            },
            "fn".to_string(),
            true,
            true,
            PackageManager::default(),
            false,
            ProgramTemplate::default(),
            TestTemplate::default(),
            false,
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Anchor workspace name must be a valid Rust identifier.")]
    fn test_init_starting_with_digit() {
        init(
            &ConfigOverride {
                cluster: None,
                wallet: None,
            },
            "1project".to_string(),
            true,
            true,
            PackageManager::default(),
            false,
            ProgramTemplate::default(),
            TestTemplate::default(),
            false,
        )
        .unwrap();
    }
}
