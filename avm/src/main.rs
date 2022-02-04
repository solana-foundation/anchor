use anyhow::{Error, Result};
use clap::{Parser, Subcommand};
use semver::Version;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[clap(name = "avm", about = "Anchor version manager")]
pub struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "Use a specific version of Anchor")]
    Use {
        #[clap(parse(try_from_str = parse_version))]
        version: Version,
    },
    #[clap(about = "Install a version of Anchor")]
    Install {
        #[clap(parse(try_from_str = parse_version))]
        version: Version,
    },
    #[clap(about = "Uninstall a version of Anchor")]
    Uninstall {
        #[clap(parse(try_from_str = parse_version))]
        version: Version,
    },
    #[clap(about = "List available versions of Anchor")]
    List {},
}

// If `latest` is passed use the latest available version.
fn parse_version(version: &str) -> Result<Version, Error> {
    if version == "latest" {
        Ok(avm::get_latest_version())
    } else {
        Version::parse(version).map_err(|e| anyhow::anyhow!(e))
    }
}
pub fn entry(opts: Cli) -> Result<()> {
    match opts.command {
        Commands::Use { version } => avm::use_version(&version),
        Commands::Install { version } => avm::install_version(&version),
        Commands::Uninstall { version } => avm::uninstall_version(&version),
        Commands::List {} => avm::list_versions(),
    }
}

fn main() -> Result<()> {
    // Make sure the user's home directory is setup with the paths required by AVM.
    avm::ensure_paths();

    let opt = Cli::parse();
    entry(opt)
}
