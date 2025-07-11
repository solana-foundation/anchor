use anyhow::{anyhow, Error, Result};
use avm::InstallTarget;
use clap::{CommandFactory, Parser, Subcommand};
use semver::Version;

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
    #[clap(about = "Install a version of Anchor", alias = "i")]
    Install {
        /// Anchor version or commit, conflicts with `--path`
        #[clap(required_unless_present = "path")]
        version_or_commit: Option<String>,
        /// Path to local anchor repo, conflicts with `version_or_commit`
        #[clap(long, conflicts_with = "version_or_commit")]
        path: Option<String>,
        #[clap(long)]
        /// Flag to force installation even if the version
        /// is already installed
        force: bool,
        #[clap(long)]
        /// Build from source code rather than downloading prebuilt binaries
        from_source: bool,
    },
    #[clap(about = "Uninstall a version of Anchor")]
    Uninstall {
        #[clap(value_parser = parse_version)]
        version: Version,
    },
    #[clap(about = "List available versions of Anchor", alias = "ls")]
    List {},
    #[clap(about = "Update to the latest Anchor version")]
    Update {},
    #[clap(about = "Generate shell completions for AVM")]
    Completions {
        #[clap(value_enum)]
        shell: clap_complete::Shell,
    },
}

// If `latest` is passed use the latest available version.
fn parse_version(version: &str) -> Result<Version, Error> {
    if version == "latest" {
        avm::get_latest_version()
    } else {
        Ok(Version::parse(version)?)
    }
}

fn parse_install_target(version_or_commit: &str) -> Result<InstallTarget, Error> {
    if let Ok(version) = parse_version(version_or_commit) {
        if version.pre.is_empty() {
            return Ok(InstallTarget::Version(version));
        }
        // Allow for e.g. `avm install 0.28.0-6cf200493a307c01487c7b492b4893e0d6f6cb23`
        return Ok(InstallTarget::Commit(version.pre.to_string()));
    }
    avm::check_and_get_full_commit(version_or_commit)
        .map(InstallTarget::Commit)
        .map_err(|e| anyhow!("Not a valid version or commit: {e}"))
}

pub fn entry(opts: Cli) -> Result<()> {
    match opts.command {
        Commands::Use { version } => avm::use_version(version),
        Commands::Install {
            version_or_commit,
            path,
            force,
            from_source,
        } => {
            let install_target = if let Some(path) = path {
                InstallTarget::Path(path.into())
            } else {
                parse_install_target(&version_or_commit.unwrap())?
            };
            avm::install_version(install_target, force, from_source)
        }
        Commands::Uninstall { version } => avm::uninstall_version(&version),
        Commands::List {} => avm::list_versions(),
        Commands::Update {} => avm::update(),
        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "avm", &mut std::io::stdout());
            Ok(())
        }
    }
}

fn main() -> Result<()> {
    // Make sure the user's home directory is setup with the paths required by AVM.
    avm::ensure_paths();

    let opt = Cli::parse();
    entry(opt)
}
