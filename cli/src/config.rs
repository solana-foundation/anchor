use anchor_client::Cluster;
use anchor_syn::idl::Idl;
use anyhow::{anyhow, Error, Result};
use clap::Clap;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Default, Debug, Clap)]
pub struct ConfigOverride {
    /// Cluster override.
    #[clap(global = true, long = "provider.cluster")]
    pub cluster: Option<Cluster>,
    /// Wallet override.
    #[clap(global = true, long = "provider.wallet")]
    pub wallet: Option<WalletPath>,
}

pub struct WithPath<T> {
    inner: T,
    path: PathBuf,
}

impl<T> WithPath<T> {
    pub fn new(inner: T, path: PathBuf) -> Self {
        Self { inner, path }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> std::convert::AsRef<T> for WithPath<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl WithPath<Config> {
    pub fn get_program_list(&self) -> Result<Vec<PathBuf>> {
        // Canonicalize the workspace filepaths to compare with relative paths.
        let (members, exclude) = self.canonicalize_workspace()?;

        // Get all candidate programs.
        //
        // If [workspace.members] exists, then use that.
        // Otherwise, default to `programs/*`.
        let program_paths: Vec<PathBuf> = {
            if members.is_empty() {
                let path = self.path().parent().unwrap().join("programs");
                fs::read_dir(path)?
                    .map(|dir| dir.map(|d| d.path().canonicalize().unwrap()))
                    .collect::<Vec<Result<PathBuf, std::io::Error>>>()
                    .into_iter()
                    .collect::<Result<Vec<PathBuf>, std::io::Error>>()?
            } else {
                members
            }
        };

        // Filter out everything part of the exclude array.
        Ok(program_paths
            .into_iter()
            .filter(|m| !exclude.contains(m))
            .collect())
    }

    // TODO: this should read idl dir instead of parsing source.
    pub fn read_all_programs(&self) -> Result<Vec<Program>> {
        let mut r = vec![];
        for path in self.get_program_list()? {
            let idl = anchor_syn::idl::file::parse(path.join("src/lib.rs"))?;
            let lib_name = extract_lib_name(&path.join("Cargo.toml"))?;
            r.push(Program {
                lib_name,
                path,
                idl,
            });
        }
        Ok(r)
    }

    pub fn canonicalize_workspace(&self) -> Result<(Vec<PathBuf>, Vec<PathBuf>)> {
        let members = self
            .workspace
            .members
            .iter()
            .map(|m| PathBuf::from(m).canonicalize().unwrap())
            .collect();
        let exclude = self
            .workspace
            .exclude
            .iter()
            .map(|m| PathBuf::from(m).canonicalize().unwrap())
            .collect();
        Ok((members, exclude))
    }
}

impl<T> std::ops::Deref for WithPath<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> std::ops::DerefMut for WithPath<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[derive(Debug, Default)]
pub struct Config {
    pub anchor_version: Option<String>,
    pub solana_version: Option<String>,
    pub registry: RegistryConfig,
    pub provider: ProviderConfig,
    pub programs: ProgramsConfig,
    pub scripts: ScriptsConfig,
    pub workspace: WorkspaceConfig,
    pub test: Option<Test>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub url: String,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            url: "https://anchor.projectserum.com".to_string(),
        }
    }
}

#[derive(Debug, Default)]
pub struct ProviderConfig {
    pub cluster: Cluster,
    pub wallet: WalletPath,
}

pub type ScriptsConfig = BTreeMap<String, String>;

pub type ProgramsConfig = BTreeMap<Cluster, BTreeMap<String, ProgramDeployment>>;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
}

impl Config {
    pub fn docker(&self) -> String {
        let ver = self
            .anchor_version
            .clone()
            .unwrap_or_else(|| crate::DOCKER_BUILDER_VERSION.to_string());
        format!("projectserum/build:v{}", ver)
    }

    pub fn discover(
        cfg_override: &ConfigOverride,
    ) -> Result<Option<(WithPath<Config>, Option<PathBuf>)>> {
        Config::_discover().map(|opt| {
            opt.map(|(mut cfg, cargo_toml)| {
                if let Some(cluster) = cfg_override.cluster.clone() {
                    cfg.provider.cluster = cluster;
                }

                if let Some(wallet) = cfg_override.wallet.clone() {
                    cfg.provider.wallet = wallet;
                }
                (cfg, cargo_toml)
            })
        })
    }

    // Searches all parent directories for an Anchor.toml file.
    fn _discover() -> Result<Option<(WithPath<Config>, Option<PathBuf>)>> {
        // Set to true if we ever see a Cargo.toml file when traversing the
        // parent directories.
        let mut cargo_toml = None;

        let _cwd = std::env::current_dir()?;
        let mut cwd_opt = Some(_cwd.as_path());

        while let Some(cwd) = cwd_opt {
            let files = fs::read_dir(cwd)?;
            // Cargo.toml file for this directory level.
            let mut cargo_toml_level = None;
            let mut anchor_toml = None;
            for f in files {
                let p = f?.path();
                if let Some(filename) = p.file_name() {
                    if filename.to_str() == Some("Cargo.toml") {
                        cargo_toml_level = Some(p);
                    } else if filename.to_str() == Some("Anchor.toml") {
                        let mut cfg_file = File::open(&p)?;
                        let mut cfg_contents = String::new();
                        cfg_file.read_to_string(&mut cfg_contents)?;
                        let cfg = cfg_contents.parse()?;
                        anchor_toml = Some((cfg, p));
                    }
                }
            }

            // Set the Cargo.toml if it's for a single package, i.e., not the
            // root workspace Cargo.toml.
            if cargo_toml.is_none() && cargo_toml_level.is_some() {
                let toml = cargo_toml::Manifest::from_path(cargo_toml_level.as_ref().unwrap())?;
                if toml.workspace.is_none() {
                    cargo_toml = cargo_toml_level;
                }
            }

            if let Some((cfg, parent)) = anchor_toml {
                return Ok(Some((WithPath::new(cfg, parent), cargo_toml)));
            }

            cwd_opt = cwd.parent();
        }

        Ok(None)
    }

    pub fn wallet_kp(&self) -> Result<Keypair> {
        solana_sdk::signature::read_keypair_file(&self.provider.wallet.to_string())
            .map_err(|_| anyhow!("Unable to read keypair file"))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct _Config {
    anchor_version: Option<String>,
    solana_version: Option<String>,
    registry: Option<RegistryConfig>,
    provider: Provider,
    test: Option<Test>,
    scripts: Option<ScriptsConfig>,
    programs: Option<BTreeMap<String, BTreeMap<String, serde_json::Value>>>,
    workspace: Option<WorkspaceConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Provider {
    cluster: String,
    wallet: String,
}

impl ToString for Config {
    fn to_string(&self) -> String {
        let programs = {
            let c = ser_programs(&self.programs);
            if c.is_empty() {
                None
            } else {
                Some(c)
            }
        };
        let cfg = _Config {
            anchor_version: self.anchor_version.clone(),
            solana_version: self.solana_version.clone(),
            registry: Some(self.registry.clone()),
            provider: Provider {
                cluster: format!("{}", self.provider.cluster),
                wallet: self.provider.wallet.to_string(),
            },
            test: self.test.clone(),
            scripts: match self.scripts.is_empty() {
                true => None,
                false => Some(self.scripts.clone()),
            },
            programs,
            workspace: (!self.workspace.members.is_empty() || !self.workspace.exclude.is_empty())
                .then(|| self.workspace.clone()),
        };

        toml::to_string(&cfg).expect("Must be well formed")
    }
}

impl FromStr for Config {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cfg: _Config = toml::from_str(s)
            .map_err(|e| anyhow::format_err!("Unable to deserialize config: {}", e.to_string()))?;
        Ok(Config {
            anchor_version: cfg.anchor_version,
            solana_version: cfg.solana_version,
            registry: cfg.registry.unwrap_or_default(),
            provider: ProviderConfig {
                cluster: cfg.provider.cluster.parse()?,
                wallet: shellexpand::tilde(&cfg.provider.wallet).parse()?,
            },
            scripts: cfg.scripts.unwrap_or_else(BTreeMap::new),
            test: cfg.test,
            programs: cfg.programs.map_or(Ok(BTreeMap::new()), deser_programs)?,
            workspace: cfg.workspace.unwrap_or_default(),
        })
    }
}

fn ser_programs(
    programs: &BTreeMap<Cluster, BTreeMap<String, ProgramDeployment>>,
) -> BTreeMap<String, BTreeMap<String, serde_json::Value>> {
    programs
        .iter()
        .map(|(cluster, programs)| {
            let cluster = cluster.to_string();
            let programs = programs
                .iter()
                .map(|(name, deployment)| {
                    (
                        name.clone(),
                        serde_json::to_value(&_ProgramDeployment::from(deployment)).unwrap(),
                    )
                })
                .collect::<BTreeMap<String, serde_json::Value>>();
            (cluster, programs)
        })
        .collect::<BTreeMap<String, BTreeMap<String, serde_json::Value>>>()
}
fn deser_programs(
    programs: BTreeMap<String, BTreeMap<String, serde_json::Value>>,
) -> Result<BTreeMap<Cluster, BTreeMap<String, ProgramDeployment>>> {
    programs
        .iter()
        .map(|(cluster, programs)| {
            let cluster: Cluster = cluster.parse()?;
            let programs = programs
                .iter()
                .map(|(name, program_id)| {
                    Ok((
                        name.clone(),
                        ProgramDeployment::try_from(match &program_id {
                            serde_json::Value::String(address) => _ProgramDeployment {
                                address: address.parse()?,
                                path: None,
                                idl: None,
                            },
                            serde_json::Value::Object(_) => {
                                serde_json::from_value(program_id.clone())
                                    .map_err(|_| anyhow!("Unable to read toml"))?
                            }
                            _ => return Err(anyhow!("Invalid toml type")),
                        })?,
                    ))
                })
                .collect::<Result<BTreeMap<String, ProgramDeployment>>>()?;
            Ok((cluster, programs))
        })
        .collect::<Result<BTreeMap<Cluster, BTreeMap<String, ProgramDeployment>>>>()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Test {
    pub genesis: Vec<GenesisEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisEntry {
    // Base58 pubkey string.
    pub address: String,
    // Filepath to the compiled program to embed into the genesis.
    pub program: String,
}

pub fn extract_lib_name(cargo_toml: impl AsRef<Path>) -> Result<String> {
    let cargo_toml = cargo_toml::Manifest::from_path(cargo_toml)?;
    if cargo_toml.lib.is_some() && cargo_toml.lib.as_ref().unwrap().name.is_some() {
        Ok(cargo_toml.lib.unwrap().name.unwrap())
    } else {
        Ok(cargo_toml
            .package
            .ok_or_else(|| anyhow!("Package section not provided"))?
            .name)
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub lib_name: String,
    pub path: PathBuf,
    pub idl: Option<Idl>,
}

impl Program {
    pub fn anchor_keypair_path(&self) -> PathBuf {
        std::env::current_dir()
            .expect("Must have current dir")
            .join(format!(
                "target/deploy/anchor-{}-keypair.json",
                self.lib_name
            ))
    }

    pub fn binary_path(&self) -> PathBuf {
        std::env::current_dir()
            .expect("Must have current dir")
            .join(format!("target/deploy/{}.so", self.lib_name))
    }
}

#[derive(Debug, Default)]
pub struct ProgramDeployment {
    pub address: Pubkey,
    pub path: Option<String>,
    pub idl: Option<String>,
}

impl TryFrom<_ProgramDeployment> for ProgramDeployment {
    type Error = anyhow::Error;
    fn try_from(pd: _ProgramDeployment) -> Result<Self, Self::Error> {
        Ok(ProgramDeployment {
            address: pd.address.parse()?,
            path: pd.path,
            idl: pd.idl,
        })
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct _ProgramDeployment {
    pub address: String,
    pub path: Option<String>,
    pub idl: Option<String>,
}

impl From<&ProgramDeployment> for _ProgramDeployment {
    fn from(pd: &ProgramDeployment) -> Self {
        Self {
            address: pd.address.to_string(),
            path: pd.path.clone(),
            idl: pd.idl.clone(),
        }
    }
}

pub struct ProgramWorkspace {
    pub name: String,
    pub program_id: Pubkey,
    pub idl: Idl,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnchorPackage {
    pub name: String,
    pub address: String,
    pub path: String,
    pub idl: Option<String>,
}

impl AnchorPackage {
    pub fn from(name: String, cfg: &WithPath<Config>) -> Result<Self> {
        let cluster = &cfg.provider.cluster;
        if cluster != &Cluster::Mainnet {
            return Err(anyhow!("Publishing requires the mainnet cluster"));
        }
        let program_details = cfg
            .programs
            .get(cluster)
            .ok_or_else(|| anyhow!("Program not provided in Anchor.toml"))?
            .get(&name)
            .ok_or_else(|| anyhow!("Program not provided in Anchor.toml"))?;
        let path = program_details
            .path
            .clone()
            // TODO: use a default path if one isn't provided?
            .ok_or_else(|| anyhow!("Path to program binary not provided"))?;
        let idl = program_details.idl.clone();
        let address = program_details.address.to_string();
        Ok(Self {
            name,
            path,
            address,
            idl,
        })
    }
}

serum_common::home_path!(WalletPath, ".config/solana/id.json");
