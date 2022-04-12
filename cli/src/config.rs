use crate::is_hidden;
use anchor_client::Cluster;
use anchor_syn::idl::Idl;
use anyhow::{anyhow, Context, Error, Result};
use clap::{ArgEnum, Parser};
use heck::SnakeCase;
use serde::{Deserialize, Serialize};
use solana_cli_config::{Config as SolanaConfig, CONFIG_FILE};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use std::collections::{BTreeMap, HashMap};
use std::convert::TryFrom;
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use walkdir::WalkDir;

pub trait Merge: Sized {
    fn merge(&mut self, _other: Self) {}
}

#[derive(Default, Debug, Parser)]
pub struct ConfigOverride {
    /// Cluster override.
    #[clap(global = true, long = "provider.cluster")]
    pub cluster: Option<Cluster>,
    /// Wallet override.
    #[clap(global = true, long = "provider.wallet")]
    pub wallet: Option<WalletPath>,
}

#[derive(Debug)]
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
        cargo_toml::Manifest::from_path(&p)
            .map(Manifest)
            .map_err(anyhow::Error::from)
            .with_context(|| format!("Error reading manifest from path: {}", p.as_ref().display()))
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
                .to_string()
                .to_snake_case())
        } else {
            Ok(self
                .package
                .as_ref()
                .ok_or_else(|| anyhow!("package section not provided"))?
                .name
                .to_string()
                .to_snake_case())
        }
    }

    pub fn version(&self) -> String {
        match &self.package {
            Some(package) => package.version.to_string(),
            _ => "0.0.0".to_string(),
        }
    }

    // Climbs each parent directory from the current dir until we find a Cargo.toml
    pub fn discover() -> Result<Option<WithPath<Manifest>>> {
        Manifest::discover_from_path(std::env::current_dir()?)
    }

    // Climbs each parent directory from a given starting directory until we find a Cargo.toml.
    pub fn discover_from_path(start_from: PathBuf) -> Result<Option<WithPath<Manifest>>> {
        let mut cwd_opt = Some(start_from.as_path());

        while let Some(cwd) = cwd_opt {
            for f in fs::read_dir(cwd).with_context(|| {
                format!("Error reading the directory with path: {}", cwd.display())
            })? {
                let p = f
                    .with_context(|| {
                        format!("Error reading the directory with path: {}", cwd.display())
                    })?
                    .path();
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
            let cargo = Manifest::from_path(&path.join("Cargo.toml"))?;
            let lib_name = cargo.lib_name()?;
            let version = cargo.version();
            let idl = anchor_syn::idl::file::parse(
                path.join("src/lib.rs"),
                version,
                self.features.seeds,
                false,
            )?;
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
                    .unwrap_or_else(|_| {
                        panic!("Error reading workspace.members. File {:?} does not exist at path {:?}.", m, self.path)
                    })
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
                    .unwrap_or_else(|_| {
                        panic!("Error reading workspace.exclude. File {:?} does not exist at path {:?}.", m, self.path)
                    })
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
    pub features: FeaturesConfig,
    pub registry: RegistryConfig,
    pub provider: ProviderConfig,
    pub programs: ProgramsConfig,
    pub scripts: ScriptsConfig,
    pub workspace: WorkspaceConfig,
    // Separate entry next to test_config because
    // "anchor localnet" only has access to the Anchor.toml,
    // not the Test.toml files
    pub test_validator: Option<TestValidator>,
    pub test_config: Option<TestConfig>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct FeaturesConfig {
    #[serde(default)]
    pub seeds: bool,
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
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub types: String,
}

#[derive(ArgEnum, Parser, Clone, PartialEq, Debug)]
pub enum BootstrapMode {
    None,
    Debian,
}

#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub verifiable: bool,
    pub solana_version: Option<String>,
    pub docker_image: String,
    pub bootstrap: BootstrapMode,
}

impl Config {
    pub fn add_test_config(&mut self, root: impl AsRef<Path>) -> Result<()> {
        self.test_config = TestConfig::discover(root)?;
        Ok(())
    }

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
            for f in fs::read_dir(cwd).with_context(|| {
                format!("Error reading the directory with path: {}", cwd.display())
            })? {
                let p = f
                    .with_context(|| {
                        format!("Error reading the directory with path: {}", cwd.display())
                    })?
                    .path();
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
        fs::read_to_string(&p)
            .with_context(|| format!("Error reading the file with path: {}", p.as_ref().display()))?
            .parse::<Self>()
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
    features: Option<FeaturesConfig>,
    programs: Option<BTreeMap<String, BTreeMap<String, serde_json::Value>>>,
    registry: Option<RegistryConfig>,
    provider: Provider,
    workspace: Option<WorkspaceConfig>,
    scripts: Option<ScriptsConfig>,
    test: Option<_TestValidator>,
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
            features: Some(self.features.clone()),
            registry: Some(self.registry.clone()),
            provider: Provider {
                cluster: format!("{}", self.provider.cluster),
                wallet: self.provider.wallet.to_string(),
            },
            test: self.test_validator.clone().map(Into::into),
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
            features: cfg.features.unwrap_or_default(),
            registry: cfg.registry.unwrap_or_default(),
            provider: ProviderConfig {
                cluster: cfg.provider.cluster.parse()?,
                wallet: shellexpand::tilde(&cfg.provider.wallet).parse()?,
            },
            scripts: cfg.scripts.unwrap_or_default(),
            test_validator: cfg.test.map(Into::into),
            test_config: None,
            programs: cfg.programs.map_or(Ok(BTreeMap::new()), deser_programs)?,
            workspace: cfg.workspace.unwrap_or_default(),
        })
    }
}

pub fn get_solana_cfg_url() -> Result<String, io::Error> {
    let config_file = CONFIG_FILE.as_ref().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Default Solana config was not found",
        )
    })?;
    SolanaConfig::load(config_file).map(|config| config.json_rpc_url)
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

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct TestValidator {
    pub genesis: Option<Vec<GenesisEntry>>,
    pub validator: Option<Validator>,
    pub startup_wait: i32,
    pub shutdown_wait: i32,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct _TestValidator {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genesis: Option<Vec<GenesisEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validator: Option<_Validator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_wait: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shutdown_wait: Option<i32>,
}

pub const STARTUP_WAIT: i32 = 5000;
pub const SHUTDOWN_WAIT: i32 = 2000;

impl From<_TestValidator> for TestValidator {
    fn from(_test_validator: _TestValidator) -> Self {
        Self {
            shutdown_wait: _test_validator.shutdown_wait.unwrap_or(SHUTDOWN_WAIT),
            startup_wait: _test_validator.startup_wait.unwrap_or(STARTUP_WAIT),
            genesis: _test_validator.genesis,
            validator: _test_validator.validator.map(Into::into),
        }
    }
}

impl From<TestValidator> for _TestValidator {
    fn from(test_validator: TestValidator) -> Self {
        Self {
            shutdown_wait: Some(test_validator.shutdown_wait),
            startup_wait: Some(test_validator.startup_wait),
            genesis: test_validator.genesis,
            validator: test_validator.validator.map(Into::into),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestConfig {
    pub test_suite_configs: HashMap<PathBuf, TestToml>,
}

impl Deref for TestConfig {
    type Target = HashMap<PathBuf, TestToml>;

    fn deref(&self) -> &Self::Target {
        &self.test_suite_configs
    }
}

impl TestConfig {
    pub fn discover(root: impl AsRef<Path>) -> Result<Option<Self>> {
        let walker = WalkDir::new(root).into_iter();
        let mut test_suite_configs = HashMap::new();
        for entry in walker.filter_entry(|e| !is_hidden(e)) {
            let entry = entry?;
            if entry.file_name() == "Test.toml" {
                let test_toml = TestToml::from_path(entry.path())?;
                test_suite_configs.insert(entry.path().into(), test_toml);
            }
        }

        Ok(match test_suite_configs.is_empty() {
            true => None,
            false => Some(Self { test_suite_configs }),
        })
    }
}

// This file needs to have the same (sub)structure as Anchor.toml
// so it can be parsed as a base test file from an Anchor.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct _TestToml {
    pub extends: Option<Vec<String>>,
    pub test: Option<_TestValidator>,
    pub scripts: Option<ScriptsConfig>,
}

impl _TestToml {
    fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let s = fs::read_to_string(&path)?;
        let parsed_toml: Self = toml::from_str(&s)?;
        let mut current_toml = _TestToml {
            extends: None,
            test: None,
            scripts: None,
        };
        if let Some(bases) = &parsed_toml.extends {
            for base in bases {
                let mut canonical_base = base.clone();
                canonical_base = canonicalize_filepath_from_origin(&canonical_base, &path)?;
                current_toml.merge(_TestToml::from_path(&canonical_base)?);
            }
        }
        current_toml.merge(parsed_toml);

        if let Some(test) = &mut current_toml.test {
            if let Some(genesis_programs) = &mut test.genesis {
                for entry in genesis_programs {
                    entry.program = canonicalize_filepath_from_origin(&entry.program, &path)?;
                }
            }
            if let Some(validator) = &mut test.validator {
                if let Some(ledger_dir) = &mut validator.ledger {
                    *ledger_dir = canonicalize_filepath_from_origin(&ledger_dir, &path)?;
                }
                if let Some(accounts) = &mut validator.account {
                    for entry in accounts {
                        entry.filename = canonicalize_filepath_from_origin(&entry.filename, &path)?;
                    }
                }
            }
        }
        Ok(current_toml)
    }
}

/// canonicalizes the `file_path` arg.
/// uses the `path` arg as the current dir
/// from which to turn the relative path
/// into a canonical one
fn canonicalize_filepath_from_origin(
    file_path: impl AsRef<Path>,
    origin: impl AsRef<Path>,
) -> Result<String> {
    let previous_dir = std::env::current_dir()?;
    std::env::set_current_dir(origin.as_ref().parent().unwrap())?;
    let result = fs::canonicalize(&file_path)
        .with_context(|| {
            format!(
                "Error reading (possibly relative) path: {}. If relative, this is the path that was used as the current path: {}",
                &file_path.as_ref().display(),
                &origin.as_ref().display()
            )
        })?
        .display()
        .to_string();
    std::env::set_current_dir(previous_dir)?;
    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestToml {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<TestValidator>,
    pub scripts: ScriptsConfig,
}

impl TestToml {
    pub fn from_path(p: impl AsRef<Path>) -> Result<Self> {
        WithPath::new(_TestToml::from_path(&p)?, p.as_ref().into()).try_into()
    }
}

impl Merge for _TestToml {
    fn merge(&mut self, other: Self) {
        let mut my_scripts = self.scripts.take();
        match &mut my_scripts {
            None => my_scripts = other.scripts,
            Some(my_scripts) => {
                if let Some(other_scripts) = other.scripts {
                    for (name, script) in other_scripts {
                        my_scripts.insert(name, script);
                    }
                }
            }
        }

        let mut my_test = self.test.take();
        match &mut my_test {
            Some(my_test) => {
                if let Some(other_test) = other.test {
                    if let Some(startup_wait) = other_test.startup_wait {
                        my_test.startup_wait = Some(startup_wait);
                    }
                    if let Some(other_genesis) = other_test.genesis {
                        match &mut my_test.genesis {
                            Some(my_genesis) => {
                                for other_entry in other_genesis {
                                    match my_genesis
                                        .iter()
                                        .position(|g| *g.address == other_entry.address)
                                    {
                                        None => my_genesis.push(other_entry),
                                        Some(i) => my_genesis[i] = other_entry,
                                    }
                                }
                            }
                            None => my_test.genesis = Some(other_genesis),
                        }
                    }
                    let mut my_validator = my_test.validator.take();
                    match &mut my_validator {
                        None => my_validator = other_test.validator,
                        Some(my_validator) => {
                            if let Some(other_validator) = other_test.validator {
                                my_validator.merge(other_validator)
                            }
                        }
                    }

                    my_test.validator = my_validator;
                }
            }
            None => my_test = other.test,
        };

        // Instantiating a new Self object here ensures that
        // this function will fail to compile if new fields get added
        // to Self. This is useful as a reminder if they also require merging
        *self = Self {
            test: my_test,
            scripts: my_scripts,
            extends: self.extends.take(),
        };
    }
}

impl TryFrom<WithPath<_TestToml>> for TestToml {
    type Error = Error;

    fn try_from(mut value: WithPath<_TestToml>) -> Result<Self, Self::Error> {
        Ok(Self {
            test: value.test.take().map(Into::into),
            scripts: value
                .scripts
                .take()
                .ok_or_else(|| anyhow!("Missing 'scripts' section in Test.toml file."))?,
        })
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountEntry {
    // Base58 pubkey string.
    pub address: String,
    // Name of JSON file containing the account data.
    pub filename: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct _Validator {
    // Load an account from the provided JSON file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<Vec<AccountEntry>>,
    // IP address to bind the validator ports. [default: 0.0.0.0]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind_address: Option<String>,
    // Copy an account from the cluster referenced by the url argument.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clone: Option<Vec<CloneEntry>>,
    // Range to use for dynamically assigned ports. [default: 1024-65535]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_port_range: Option<String>,
    // Enable the faucet on this port [default: 9900].
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ledger: Option<String>,
    // Keep this amount of shreds in root slots. [default: 10000]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_ledger_size: Option<String>,
    // Enable JSON RPC on this port, and the next port for the RPC websocket. [default: 8899]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpc_port: Option<u16>,
    // Override the number of slots in an epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slots_per_epoch: Option<String>,
    // Warp the ledger to WARP_SLOT after starting the validator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warp_slot: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Validator {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<Vec<AccountEntry>>,
    pub bind_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clone: Option<Vec<CloneEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_port_range: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faucet_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faucet_sol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gossip_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gossip_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub ledger: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_ledger_size: Option<String>,
    pub rpc_port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slots_per_epoch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warp_slot: Option<String>,
}

impl From<_Validator> for Validator {
    fn from(_validator: _Validator) -> Self {
        Self {
            account: _validator.account,
            bind_address: _validator
                .bind_address
                .unwrap_or_else(|| DEFAULT_BIND_ADDRESS.to_string()),
            clone: _validator.clone,
            dynamic_port_range: _validator.dynamic_port_range,
            faucet_port: _validator.faucet_port,
            faucet_sol: _validator.faucet_sol,
            gossip_host: _validator.gossip_host,
            gossip_port: _validator.gossip_port,
            url: _validator.url,
            ledger: _validator
                .ledger
                .unwrap_or_else(|| DEFAULT_LEDGER_PATH.to_string()),
            limit_ledger_size: _validator.limit_ledger_size,
            rpc_port: _validator
                .rpc_port
                .unwrap_or(solana_sdk::rpc_port::DEFAULT_RPC_PORT),
            slots_per_epoch: _validator.slots_per_epoch,
            warp_slot: _validator.warp_slot,
        }
    }
}

impl From<Validator> for _Validator {
    fn from(validator: Validator) -> Self {
        Self {
            account: validator.account,
            bind_address: Some(validator.bind_address),
            clone: validator.clone,
            dynamic_port_range: validator.dynamic_port_range,
            faucet_port: validator.faucet_port,
            faucet_sol: validator.faucet_sol,
            gossip_host: validator.gossip_host,
            gossip_port: validator.gossip_port,
            url: validator.url,
            ledger: Some(validator.ledger),
            limit_ledger_size: validator.limit_ledger_size,
            rpc_port: Some(validator.rpc_port),
            slots_per_epoch: validator.slots_per_epoch,
            warp_slot: validator.warp_slot,
        }
    }
}

const DEFAULT_LEDGER_PATH: &str = ".anchor/test-ledger";
const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0";

impl Merge for _Validator {
    fn merge(&mut self, other: Self) {
        // Instantiating a new Self object here ensures that
        // this function will fail to compile if new fields get added
        // to Self. This is useful as a reminder if they also require merging
        *self = Self {
            account: match self.account.take() {
                None => other.account,
                Some(mut entries) => match other.account {
                    None => Some(entries),
                    Some(other_entries) => {
                        for other_entry in other_entries {
                            match entries
                                .iter()
                                .position(|my_entry| *my_entry.address == other_entry.address)
                            {
                                None => entries.push(other_entry),
                                Some(i) => entries[i] = other_entry,
                            };
                        }
                        Some(entries)
                    }
                },
            },
            bind_address: other.bind_address.or_else(|| self.bind_address.take()),
            clone: match self.clone.take() {
                None => other.clone,
                Some(mut entries) => match other.clone {
                    None => Some(entries),
                    Some(other_entries) => {
                        for other_entry in other_entries {
                            match entries
                                .iter()
                                .position(|my_entry| *my_entry.address == other_entry.address)
                            {
                                None => entries.push(other_entry),
                                Some(i) => entries[i] = other_entry,
                            };
                        }
                        Some(entries)
                    }
                },
            },
            dynamic_port_range: other
                .dynamic_port_range
                .or_else(|| self.dynamic_port_range.take()),
            faucet_port: other.faucet_port.or_else(|| self.faucet_port.take()),
            faucet_sol: other.faucet_sol.or_else(|| self.faucet_sol.take()),
            gossip_host: other.gossip_host.or_else(|| self.gossip_host.take()),
            gossip_port: other.gossip_port.or_else(|| self.gossip_port.take()),
            url: other.url.or_else(|| self.url.take()),
            ledger: other.ledger.or_else(|| self.ledger.take()),
            limit_ledger_size: other
                .limit_ledger_size
                .or_else(|| self.limit_ledger_size.take()),
            rpc_port: other.rpc_port.or_else(|| self.rpc_port.take()),
            slots_per_epoch: other
                .slots_per_epoch
                .or_else(|| self.slots_per_epoch.take()),
            warp_slot: other.warp_slot.or_else(|| self.warp_slot.take()),
        };
    }
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
        let deploy_dir_path = "target/deploy/";
        fs::create_dir_all(deploy_dir_path)
            .with_context(|| format!("Error creating directory with path: {}", deploy_dir_path))?;
        let path = std::env::current_dir()
            .expect("Must have current dir")
            .join(format!("target/deploy/{}-keypair.json", self.lib_name));
        if path.exists() {
            return Ok(WithPath::new(
                File::open(&path)
                    .with_context(|| format!("Error opening file with path: {}", path.display()))?,
                path,
            ));
        }
        let program_kp = Keypair::generate(&mut rand::rngs::OsRng);
        let mut file = File::create(&path)
            .with_context(|| format!("Error creating file with path: {}", path.display()))?;
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

crate::home_path!(WalletPath, ".config/solana/id.json");
