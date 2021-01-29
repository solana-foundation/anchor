use anchor_syn::idl::Idl;
use anyhow::{anyhow, Error, Result};
use serde::{Deserialize, Serialize};
use serum_common::client::Cluster;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Default)]
pub struct Config {
    pub cluster: Cluster,
    pub wallet: WalletPath,
}

impl Config {
    // Searches all parent directories for an Anchor.toml file.
    pub fn discover() -> Result<Option<(Self, PathBuf, Option<PathBuf>)>> {
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
                        cargo_toml_level = Some(PathBuf::from(p));
                    } else if filename.to_str() == Some("Anchor.toml") {
                        let mut cfg_file = File::open(&p)?;
                        let mut cfg_contents = String::new();
                        cfg_file.read_to_string(&mut cfg_contents)?;
                        let cfg = cfg_contents.parse()?;
                        anchor_toml = Some((cfg, PathBuf::from(p)));
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
}

// Pubkey serializes as a byte array so use this type a hack to serialize
// into base 58 strings.
#[derive(Serialize, Deserialize)]
struct _Config {
    cluster: String,
    wallet: String,
}

impl ToString for Config {
    fn to_string(&self) -> String {
        let cfg = _Config {
            cluster: format!("{}", self.cluster),
            wallet: self.wallet.to_string(),
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
            cluster: cfg.cluster.parse()?,
            wallet: shellexpand::tilde(&cfg.wallet).parse()?,
        })
    }
}

pub fn find_cargo_toml() -> Result<Option<PathBuf>> {
    let _cwd = std::env::current_dir()?;
    let mut cwd_opt = Some(_cwd.as_path());
    while let Some(cwd) = cwd_opt {
        let files = fs::read_dir(cwd)?;
        for f in files {
            let p = f?.path();
            if let Some(filename) = p.file_name() {
                if filename.to_str() == Some("Cargo.toml") {
                    return Ok(Some(PathBuf::from(p)));
                }
            }
        }
        cwd_opt = cwd.parent();
    }
    Ok(None)
}

pub fn read_all_programs() -> Result<Vec<Program>> {
    let files = fs::read_dir("programs")?;
    let mut r = vec![];
    for f in files {
        let path = f?.path();
        let idl = anchor_syn::parser::file::parse(path.join("src/lib.rs"))?;
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
                .ok_or(anyhow!("lib name not found in Cargo.toml"))?
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

serum_common::home_path!(WalletPath, ".config/solana/id.json");
