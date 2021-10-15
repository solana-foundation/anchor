use anchor_client::Cluster;
use anchor_syn::idl::Idl;
use anyhow::{anyhow, Error, Result};
use clap::Clap;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::fs::{self, File};
use std::io::prelude::*;
use std::ops::Deref;
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

#[derive(Debug, Clone, PartialEq)]
pub struct Manifest(cargo_toml::Manifest);

impl Manifest {
    pub fn from_path(p: impl AsRef<Path>) -> Result<Self> {
        cargo_toml::Manifest::from_path(p)
            .map(Manifest)
            .map_err(Into::into)
    }

    pub fn lib_name(&self) -> Result<String> {
        if self.lib.is_some() && self.lib.as_ref().unwrap().name.is_some() {
            Ok(self
                .lib
                .as_ref()
                .unwrap()
                .name
                .as_ref()
                .unwrap()
                .to_string())
        } else {
            Ok(self
                .package
                .as_ref()
                .ok_or_else(|| anyhow!("package section not provided"))?
                .name
                .to_string())
        }
    }

    // Climbs each parent directory until we find a Cargo.toml.
    pub fn discover() -> Result<Option<WithPath<Manifest>>> {
        let _cwd = std::env::current_dir()?;
        let mut cwd_opt = Some(_cwd.as_path());

        while let Some(cwd) = cwd_opt {
            for f in fs::read_dir(cwd)? {
                let p = f?.path();
                if let Some(filename) = p.file_name() {
                    if filename.to_str() == Some("Cargo.toml") {
                        let m = WithPath::new(Manifest::from_path(&p)?, p);
                        return Ok(Some(m));
                    }
                }
            }

            // Not found. Go up a directory level.
            cwd_opt = cwd.parent();
        }

        Ok(None)
    }
}

impl Deref for Manifest {
    type Target = cargo_toml::Manifest;

    fn deref(&self) -> &Self::Target {
        &self.0
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
                    .filter(|entry| entry.as_ref().map(|e| e.path().is_dir()).unwrap_or(false))
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
            let lib_name = Manifest::from_path(&path.join("Cargo.toml"))?.lib_name()?;
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
            .map(|m| {
                self.path()
                    .parent()
                    .unwrap()
                    .join(m)
                    .canonicalize()
                    .unwrap()
            })
            .collect();
        let exclude = self
            .workspace
            .exclude
            .iter()
            .map(|m| {
                self.path()
                    .parent()
                    .unwrap()
                    .join(m)
                    .canonicalize()
                    .unwrap()
            })
            .collect();
        Ok((members, exclude))
    }

    pub fn get_program(&self, name: &str) -> Result<Option<WithPath<Program>>> {
        for program in self.read_all_programs()? {
            let cargo_toml = program.path.join("Cargo.toml");
            if !cargo_toml.exists() {
                return Err(anyhow!(
                    "Did not find Cargo.toml at the path: {}",
                    program.path.display()
                ));
            }
            let p_lib_name = Manifest::from_path(&cargo_toml)?.lib_name()?;
            if name == p_lib_name {
                let path = self
                    .path()
                    .parent()
                    .unwrap()
                    .canonicalize()?
                    .join(&program.path);
                return Ok(Some(WithPath::new(program, path)));
            }
        }
        Ok(None)
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

    pub fn discover(cfg_override: &ConfigOverride) -> Result<Option<WithPath<Config>>> {
        Config::_discover().map(|opt| {
            opt.map(|mut cfg| {
                if let Some(cluster) = cfg_override.cluster.clone() {
                    cfg.provider.cluster = cluster;
                }
                if let Some(wallet) = cfg_override.wallet.clone() {
                    cfg.provider.wallet = wallet;
                }
                cfg
            })
        })
    }

    // Climbs each parent directory until we find an Anchor.toml.
    fn _discover() -> Result<Option<WithPath<Config>>> {
        let _cwd = std::env::current_dir()?;
        let mut cwd_opt = Some(_cwd.as_path());

        while let Some(cwd) = cwd_opt {
            for f in fs::read_dir(cwd)? {
                let p = f?.path();
                if let Some(filename) = p.file_name() {
                    if filename.to_str() == Some("Anchor.toml") {
                        let cfg = Config::from_path(&p)?;
                        return Ok(Some(WithPath::new(cfg, p)));
                    }
                }
            }

            cwd_opt = cwd.parent();
        }

        Ok(None)
    }

    fn from_path(p: impl AsRef<Path>) -> Result<Self> {
        let mut cfg_file = File::open(&p)?;
        let mut cfg_contents = String::new();
        cfg_file.read_to_string(&mut cfg_contents)?;
        let cfg = cfg_contents.parse()?;

        Ok(cfg)
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
    programs: Option<BTreeMap<String, BTreeMap<String, serde_json::Value>>>,
    registry: Option<RegistryConfig>,
    provider: Provider,
    workspace: Option<WorkspaceConfig>,
    scripts: Option<ScriptsConfig>,
    test: Option<Test>,
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
                        to_value(&_ProgramDeployment::from(deployment)),
                    )
                })
                .collect::<BTreeMap<String, serde_json::Value>>();
            (cluster, programs)
        })
        .collect::<BTreeMap<String, BTreeMap<String, serde_json::Value>>>()
}

fn to_value(dep: &_ProgramDeployment) -> serde_json::Value {
    if dep.path.is_none() && dep.idl.is_none() {
        return serde_json::Value::String(dep.address.to_string());
    }
    serde_json::to_value(dep).unwrap()
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
    pub genesis: Option<Vec<GenesisEntry>>,
    pub clone: Option<Vec<CloneEntry>>,
    pub validator: Option<Validator>,
    pub startup_wait: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisEntry {
    // Base58 pubkey string.
    pub address: String,
    // Filepath to the compiled program to embed into the genesis.
    pub program: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneEntry {
    // Base58 pubkey string.
    pub address: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Validator {
    // IP address to bind the validator ports. [default: 0.0.0.0]
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    // Range to use for dynamically assigned ports. [default: 1024-65535]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_port_range: Option<String>,
    // Enable the faucet on this port [deafult: 9900].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faucet_port: Option<u16>,
    // Give the faucet address this much SOL in genesis. [default: 1000000]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faucet_sol: Option<String>,
    // Gossip DNS name or IP address for the validator to advertise in gossip. [default: 127.0.0.1]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gossip_host: Option<String>,
    // Gossip port number for the validator
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gossip_port: Option<u16>,
    // URL for Solana's JSON RPC or moniker.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    // Use DIR as ledger location
    #[serde(default = "default_ledger_path")]
    pub ledger: String,
    // Keep this amount of shreds in root slots. [default: 10000]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_ledger_size: Option<String>,
    // Enable JSON RPC on this port, and the next port for the RPC websocket. [default: 8899]
    #[serde(default = "default_rpc_port")]
    pub rpc_port: u16,
    // Override the number of slots in an epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slots_per_epoch: Option<String>,
    // Warp the ledger to WARP_SLOT after starting the validator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warp_slot: Option<String>,
}

fn default_ledger_path() -> String {
    ".anchor/test-ledger".to_string()
}

fn default_bind_address() -> String {
    "0.0.0.0".to_string()
}

fn default_rpc_port() -> u16 {
    8899
}

#[derive(Debug, Clone)]
pub struct Program {
    pub lib_name: String,
    // Canonicalized path to the program directory.
    pub path: PathBuf,
    pub idl: Option<Idl>,
}

impl Program {
    pub fn pubkey(&self) -> Result<Pubkey> {
        self.keypair().map(|kp| kp.pubkey())
    }

    pub fn keypair(&self) -> Result<Keypair> {
        let file = self.keypair_file()?;
        solana_sdk::signature::read_keypair_file(file.path())
            .map_err(|_| anyhow!("failed to read keypair for program: {}", self.lib_name))
    }

    // Lazily initializes the keypair file with a new key if it doesn't exist.
    pub fn keypair_file(&self) -> Result<WithPath<File>> {
        fs::create_dir_all("target/deploy/")?;
        let path = std::env::current_dir()
            .expect("Must have current dir")
            .join(format!("target/deploy/{}-keypair.json", self.lib_name));
        if path.exists() {
            return Ok(WithPath::new(File::open(&path)?, path));
        }
        let program_kp = Keypair::generate(&mut rand::rngs::OsRng);
        let mut file = File::create(&path)?;
        file.write_all(format!("{:?}", &program_kp.to_bytes()).as_bytes())?;
        Ok(WithPath::new(file, path))
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
        let idl = program_details.idl.clone();
        let address = program_details.address.to_string();
        Ok(Self { name, address, idl })
    }
}

serum_common::home_path!(WalletPath, ".config/solana/id.json");
