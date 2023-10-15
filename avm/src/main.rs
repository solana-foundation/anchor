use anyhow::{anyhow, Error, Result};
use avm::InstallTarget;
use clap::{Parser, Subcommand};
use semver::Version;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[clap(name = "avm", about = "Anchor version manager", version)]
pub struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "Use a specific version of Anchor")]
    Use {
        #[clap(value_parser = parse_version, required = false)]
        version: Option<Version>,
    },
    #[clap(about = "Install a version of Anchor")]
    Install {
        /// Anchor version or commit
        #[clap(value_parser = parse_install_target)]
        version_or_commit: InstallTarget,
        #[clap(long)]
        /// Flag to force installation even if the version
        /// is already installed
        force: bool,
    },
    #[clap(about = "Uninstall a version of Anchor")]
    Uninstall {
        #[clap(value_parser = parse_version)]
        version: Version,
    },
    #[clap(about = "List available versions of Anchor")]
    List {},
    #[clap(about = "Update to the latest Anchor version")]
    Update {},
}

// If `latest` is passed use the latest available version.
fn parse_version(version: &str) -> Result<Version, Error> {
    if version == "latest" {
        avm::get_latest_version()
    } else {
        Version::parse(version).map_err(|e| anyhow!(e))
    }
}

fn parse_install_target(version_or_commit: &str) -> Result<InstallTarget, Error> {
    parse_version(version_or_commit)
        .map(|version| {
            if version.pre.is_empty() {
                InstallTarget::Version(version)
            } else {
                // Allow `avm install 0.28.0-6cf200493a307c01487c7b492b4893e0d6f6cb23`
                InstallTarget::Commit(version.pre.to_string())
            }
        })
        .or_else(|version_error| {
            avm::check_and_get_full_commit(version_or_commit)
                .map(InstallTarget::Commit)
                .map_err(|commit_error| {
                    anyhow!("Not a valid version or commit: {version_error}, {commit_error}")
                })
        })
}

pub fn entry(opts: Cli) -> Result<()> {
    match opts.command {
        Commands::Use { version } => avm::use_version(version),
        Commands::Install {
            version_or_commit,
            force,
        } => avm::install_version(version_or_commit, force),
        Commands::Uninstall { version } => avm::uninstall_version(&version),
        Commands::List {} => avm::list_versions(),
        Commands::Update {} => avm::update(),
    }
}

fn main() -> Result<()> {
    // Make sure the user's home directory is setup with the paths required by AVM.
    avm::ensure_paths();

    let opt = Cli::parse();
    entry(opt)
}
