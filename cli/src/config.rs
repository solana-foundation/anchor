use crate::ConfigOverride;
use anchor_client::Cluster;
use anchor_syn::idl::Idl;
use anyhow::{anyhow, Error, Result};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Default)]
pub struct Config {
    pub provider: ProviderConfig,
    pub clusters: ClustersConfig,
    pub test: Option<Test>,
}

#[derive(Debug, Default)]
pub struct ProviderConfig {
    pub cluster: Cluster,
    pub wallet: WalletPath,
}

pub type ClustersConfig = BTreeMap<Cluster, BTreeMap<String, ProgramDeployment>>;

impl Config {
    pub fn discover(
        cfg_override: &ConfigOverride,
    ) -> Result<Option<(Self, PathBuf, Option<PathBuf>)>> {
        Config::_discover().map(|opt| {
            opt.map(|(mut cfg, cfg_path, cargo_toml)| {
                if let Some(cluster) = cfg_override.cluster.clone() {
                    cfg.provider.cluster = cluster;
                }

                if let Some(wallet) = cfg_override.wallet.clone() {
                    cfg.provider.wallet = wallet;
                }
                (cfg, cfg_path, cargo_toml)
            })
        })
    }

    // Searches all parent directories for an Anchor.toml file.
    fn _discover() -> Result<Option<(Self, PathBuf, Option<PathBuf>)>> {
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

            if let Some((cfg, parent)) = anchor_toml {
                return Ok(Some((cfg, parent, cargo_toml)));
            }

            if cargo_toml.is_none() {
                cargo_toml = cargo_toml_level;
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

// Pubkey serializes as a byte array so use this type a hack to serialize
// into base 58 strings.
#[derive(Debug, Serialize, Deserialize)]
struct _Config {
    provider: Provider,
    test: Option<Test>,
    clusters: Option<BTreeMap<String, BTreeMap<String, String>>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Provider {
    cluster: String,
    wallet: String,
}

impl ToString for Config {
    fn to_string(&self) -> String {
        let clusters = {
            let c = ser_clusters(&self.clusters);
            if c.len() == 0 {
                None
            } else {
                Some(c)
            }
        };
        let cfg = _Config {
            provider: Provider {
                cluster: format!("{}", self.provider.cluster),
                wallet: self.provider.wallet.to_string(),
            },
            test: self.test.clone(),
            clusters,
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
            provider: ProviderConfig {
                cluster: cfg.provider.cluster.parse()?,
                wallet: shellexpand::tilde(&cfg.provider.wallet).parse()?,
            },
            test: cfg.test,
            clusters: cfg
                .clusters
                .map_or(Ok(BTreeMap::new()), |c| deser_clusters(c))?,
        })
    }
}

fn ser_clusters(
    clusters: &BTreeMap<Cluster, BTreeMap<String, ProgramDeployment>>,
) -> BTreeMap<String, BTreeMap<String, String>> {
    clusters
        .iter()
        .map(|(cluster, programs)| {
            let cluster = cluster.to_string();
            let programs = programs
                .iter()
                .map(|(name, deployment)| (name.clone(), deployment.program_id.to_string()))
                .collect::<BTreeMap<String, String>>();
            (cluster, programs)
        })
        .collect::<BTreeMap<String, BTreeMap<String, String>>>()
}

fn deser_clusters(
    clusters: BTreeMap<String, BTreeMap<String, String>>,
) -> Result<BTreeMap<Cluster, BTreeMap<String, ProgramDeployment>>> {
    clusters
        .iter()
        .map(|(cluster, programs)| {
            let cluster: Cluster = cluster.parse()?;
            let programs = programs
                .iter()
                .map(|(name, program_id)| {
                    Ok((
                        name.clone(),
                        ProgramDeployment {
                            name: name.clone(),
                            program_id: program_id.parse()?,
                        },
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

// TODO: this should read idl dir instead of parsing source.
pub fn read_all_programs() -> Result<Vec<Program>> {
    let files = fs::read_dir("programs")?;
    let mut r = vec![];
    for f in files {
        let path = f?.path();
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

pub fn extract_lib_name(path: impl AsRef<Path>) -> Result<String> {
    let mut toml = File::open(path)?;
    let mut contents = String::new();
    toml.read_to_string(&mut contents)?;

    let cargo_toml: toml::Value = contents.parse()?;

    match cargo_toml {
        toml::Value::Table(t) => match t.get("lib") {
            None => Err(anyhow!("lib not found in Cargo.toml")),
            Some(lib) => match lib
                .get("name")
                .ok_or_else(|| anyhow!("lib name not found in Cargo.toml"))?
            {
                toml::Value::String(n) => Ok(n.to_string()),
                _ => Err(anyhow!("lib name must be a string")),
            },
        },
        _ => Err(anyhow!("Invalid Cargo.toml")),
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub lib_name: String,
    pub path: PathBuf,
    pub idl: Idl,
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
    pub name: String,
    pub program_id: Pubkey,
}

pub struct ProgramWorkspace {
    pub name: String,
    pub program_id: Pubkey,
    pub idl: Idl,
}

serum_common::home_path!(WalletPath, ".config/solana/id.json");
