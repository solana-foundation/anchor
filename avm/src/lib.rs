use anyhow::{anyhow, Error, Result};
use cargo_toml::Manifest;
use chrono::{TimeZone, Utc};
use reqwest::header::USER_AGENT;
use reqwest::StatusCode;
use semver::{Prerelease, Version};
use serde::{de, Deserialize};
use std::fs;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::LazyLock;

/// Storage directory for AVM, customizable by setting the $AVM_HOME, defaults to ~/.avm
pub static AVM_HOME: LazyLock<PathBuf> = LazyLock::new(|| {
    cfg_if::cfg_if! {
        if #[cfg(test)] {
            let dir = tempfile::tempdir().expect("Could not create temporary directory");
            dir.path().join(".avm")
        } else {
            if let Ok(avm_home) = std::env::var("AVM_HOME") {
                PathBuf::from(avm_home)
            } else {
                let mut user_home = dirs::home_dir().expect("Could not find home directory");
                user_home.push(".avm");
                user_home
            }
        }
    }
});

/// Path to the current version file $AVM_HOME/.version
fn current_version_file_path() -> PathBuf {
    AVM_HOME.join(".version")
}

/// Path to the current version file $AVM_HOME/bin
fn get_bin_dir_path() -> PathBuf {
    AVM_HOME.join("bin")
}

/// Path to the binary for the given version
pub fn version_binary_path(version: &Version) -> PathBuf {
    get_bin_dir_path().join(format!("anchor-{version}"))
}

/// Ensure the users home directory is setup with the paths required by AVM.
pub fn ensure_paths() {
    let home_dir = AVM_HOME.to_path_buf();
    if !home_dir.exists() {
        fs::create_dir_all(&home_dir).expect("Could not create .avm directory");
    }

    let bin_dir = get_bin_dir_path();
    if !bin_dir.exists() {
        fs::create_dir_all(bin_dir).expect("Could not create .avm/bin directory");
    }

    if !current_version_file_path().exists() {
        fs::File::create(current_version_file_path()).expect("Could not create .version file");
    }
}

/// Read the current version from the version file
pub fn current_version() -> Result<Version> {
    fs::read_to_string(current_version_file_path())
        .map_err(|e| anyhow!("Could not read version file: {}", e))?
        .trim_end_matches('\n')
        .parse::<Version>()
        .map_err(|e| anyhow!("Could not parse version file: {}", e))
}

/// Update the current version to a new version
pub fn use_version(opt_version: Option<Version>) -> Result<()> {
    let version = match opt_version {
        Some(version) => version,
        None => read_anchorversion_file()?,
    };

    // Make sure the requested version is installed
    let installed_versions = read_installed_versions()?;
    if !installed_versions.contains(&version) {
        println!("Version {version} is not installed. Would you like to install? [y/n]");
        let input = std::io::stdin()
            .lock()
            .lines()
            .next()
            .expect("Expected input")?;
        match input.as_str() {
            "y" | "yes" => return install_version(InstallTarget::Version(version), false, false),
            _ => return Err(anyhow!("Installation rejected.")),
        };
    }

    let mut current_version_file = fs::File::create(current_version_file_path())?;
    current_version_file.write_all(version.to_string().as_bytes())?;
    println!("Now using anchor version {}.", current_version()?);
    Ok(())
}

#[derive(Clone)]
pub enum InstallTarget {
    Version(Version),
    Commit(String),
}

/// Update to the latest version
pub fn update() -> Result<()> {
    let latest_version = get_latest_version()?;
    install_version(InstallTarget::Version(latest_version), false, false)
}

/// The commit sha provided can be shortened,
///
/// returns the full commit sha3 for unique versioning downstream
pub fn check_and_get_full_commit(commit: &str) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(format!(
            "https://api.github.com/repos/coral-xyz/anchor/commits/{commit}"
        ))
        .header(USER_AGENT, "avm https://github.com/coral-xyz/anchor")
        .send()?;

    if response.status() != StatusCode::OK {
        return Err(anyhow!(
            "Error checking commit {commit}: {}",
            response.text()?
        ));
    };

    #[derive(Deserialize)]
    struct GetCommitResponse {
        sha: String,
    }

    response
        .json::<GetCommitResponse>()
        .map(|resp| resp.sha)
        .map_err(|err| anyhow!("Failed to parse the response to JSON: {err:?}"))
}

fn get_anchor_version_from_commit(commit: &str) -> Result<Version> {
    // We read the version from cli/Cargo.toml since there is no simpler way to do so
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(format!(
            "https://raw.githubusercontent.com/coral-xyz/anchor/{commit}/cli/Cargo.toml"
        ))
        .header(USER_AGENT, "avm https://github.com/coral-xyz/anchor")
        .send()?;

    if response.status() != StatusCode::OK {
        return Err(anyhow!(
            "Could not find anchor-cli version for commit: {response:?}"
        ));
    };

    let anchor_cli_cargo_toml = response.text()?;
    let anchor_cli_manifest = Manifest::from_str(&anchor_cli_cargo_toml)?;
    let mut version = anchor_cli_manifest.package().version().parse::<Version>()?;
    version.pre = Prerelease::new(commit)?;

    Ok(version)
}

/// Install a version of anchor-cli
pub fn install_version(
    install_target: InstallTarget,
    force: bool,
    from_source: bool,
) -> Result<()> {
    let version = match &install_target {
        InstallTarget::Version(version) => version.to_owned(),
        InstallTarget::Commit(commit) => get_anchor_version_from_commit(commit)?,
    };
    // Return early if version is already installed
    if !force && read_installed_versions()?.contains(&version) {
        eprintln!("Version `{version}` is already installed");
        return Ok(());
    }

    let is_commit = matches!(install_target, InstallTarget::Commit(_));
    let is_older_than_v0_31_0 = version < Version::parse("0.31.0")?;
    if from_source || is_commit || is_older_than_v0_31_0 {
        // Build from source using `cargo install --git`
        let mut args: Vec<String> = vec![
            "install".into(),
            "anchor-cli".into(),
            "--git".into(),
            "https://github.com/coral-xyz/anchor".into(),
            "--locked".into(),
            "--root".into(),
            AVM_HOME.to_str().unwrap().into(),
        ];
        let conditional_args = match install_target {
            InstallTarget::Version(version) => ["--tag".into(), format!("v{}", version)],
            InstallTarget::Commit(commit) => ["--rev".into(), commit],
        };
        args.extend_from_slice(&conditional_args);

        // If the version is older than v0.31, install using `rustc 1.79.0` to get around the problem
        // explained in https://github.com/coral-xyz/anchor/pull/3143
        if is_older_than_v0_31_0 {
            const REQUIRED_VERSION: &str = "1.79.0";
            let is_installed = Command::new("rustup")
                .args(["toolchain", "list"])
                .output()
                .map(|output| String::from_utf8(output.stdout))??
                .lines()
                .any(|line| line.starts_with(REQUIRED_VERSION));
            if !is_installed {
                let exit_status = Command::new("rustup")
                    .args(["toolchain", "install", REQUIRED_VERSION])
                    .spawn()?
                    .wait()?;
                if !exit_status.success() {
                    return Err(anyhow!(
                        "Installation of `rustc {REQUIRED_VERSION}` failed. \
                    `rustc <1.80` is required to install Anchor v{version} from source. \
                    See https://github.com/coral-xyz/anchor/pull/3143 for more information."
                    ));
                }
            }

            // Prepend the toolchain to use with the `cargo install` command
            args.insert(0, format!("+{REQUIRED_VERSION}"));
        }

        let output = Command::new("cargo")
            .args(args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| anyhow!("`cargo install` for version `{version}` failed: {e}"))?;
        if !output.status.success() {
            return Err(anyhow!(
                "Failed to install {version}, is it a valid version?"
            ));
        }

        let bin_dir = get_bin_dir_path();
        let bin_name = if cfg!(target_os = "windows") {
            "anchor.exe"
        } else {
            "anchor"
        };
        fs::rename(bin_dir.join(bin_name), version_binary_path(&version))?;
    } else {
        let output = Command::new("rustc").arg("-vV").output()?;
        let target = core::str::from_utf8(&output.stdout)?
            .lines()
            .find(|line| line.starts_with("host:"))
            .and_then(|line| line.split(':').last())
            .ok_or_else(|| anyhow!("`host` not found from `rustc -vV` output"))?
            .trim();
        let ext = if cfg!(target_os = "windows") {
            ".exe"
        } else {
            ""
        };
        let res = reqwest::blocking::get(format!(
            "https://github.com/coral-xyz/anchor/releases/download/v{version}/anchor-{version}-{target}{ext}"
        ))?;
        if !res.status().is_success() {
            return Err(anyhow!(
                "Failed to download the binary for version `{version}` (status code: {})",
                res.status()
            ));
        }

        let bin_path = version_binary_path(&version);
        fs::write(&bin_path, res.bytes()?)?;

        // Set file to executable on UNIX
        #[cfg(unix)]
        fs::set_permissions(
            bin_path,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o775),
        )?;
    }

    // If .version file is empty or not parseable, write the newly installed version to it
    if current_version().is_err() {
        let mut current_version_file = fs::File::create(current_version_file_path())?;
        current_version_file.write_all(version.to_string().as_bytes())?;
    }

    use_version(Some(version))
}

/// Remove an installed version of anchor-cli
pub fn uninstall_version(version: &Version) -> Result<()> {
    let version_path = version_binary_path(version);
    if !version_path.exists() {
        return Err(anyhow!("anchor-cli {} is not installed", version));
    }
    if version == &current_version()? {
        return Err(anyhow!("anchor-cli {} is currently in use", version));
    }
    fs::remove_file(version_path)?;

    Ok(())
}

/// Read version from .anchorversion
pub fn read_anchorversion_file() -> Result<Version> {
    fs::read_to_string(".anchorversion")
        .map_err(|e| anyhow!(".anchorversion file not found: {e}"))
        .map(|content| Version::parse(content.trim()))?
        .map_err(|e| anyhow!("Unable to parse version: {e}"))
}

/// Retrieve a list of installable versions of anchor-cli using the GitHub API and tags on the Anchor
/// repository.
pub fn fetch_versions() -> Result<Vec<Version>, Error> {
    #[derive(Deserialize)]
    struct Release {
        #[serde(rename = "name", deserialize_with = "version_deserializer")]
        version: Version,
    }

    fn version_deserializer<'de, D>(deserializer: D) -> Result<Version, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s: &str = de::Deserialize::deserialize(deserializer)?;
        Version::parse(s.trim_start_matches('v')).map_err(de::Error::custom)
    }

    let response = reqwest::blocking::Client::new()
        .get("https://api.github.com/repos/coral-xyz/anchor/tags")
        .header(USER_AGENT, "avm https://github.com/coral-xyz/anchor")
        .send()?;

    if response.status().is_success() {
        let releases: Vec<Release> = response.json()?;
        let versions = releases.into_iter().map(|r| r.version).collect();
        Ok(versions)
    } else {
        let reset_time_header = response
            .headers()
            .get("X-RateLimit-Reset")
            .map_or("unknown", |v| v.to_str().unwrap());
        let t = Utc.timestamp_opt(reset_time_header.parse::<i64>().unwrap(), 0);
        let reset_time = t
            .single()
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "unknown".to_string());
        Err(anyhow!(
            "GitHub API rate limit exceeded. Try again after {} UTC.",
            reset_time
        ))
    }
}

/// Print available versions and flags indicating installed, current and latest
pub fn list_versions() -> Result<()> {
    let mut installed_versions = read_installed_versions()?;

    let mut available_versions = fetch_versions()?;
    // Reverse version list so latest versions are printed last
    available_versions.reverse();

    let print_versions =
        |versions: Vec<Version>, installed_versions: &mut Vec<Version>, show_latest: bool| {
            versions.iter().enumerate().for_each(|(i, v)| {
                print!("{v}");
                let mut flags = vec![];
                if i == versions.len() - 1 && show_latest {
                    flags.push("latest");
                }
                if let Some(position) = installed_versions.iter().position(|iv| iv == v) {
                    flags.push("installed");
                    installed_versions.remove(position);
                }
                if current_version().map(|cv| &cv == v).unwrap_or_default() {
                    flags.push("current");
                }

                if flags.is_empty() {
                    println!();
                } else {
                    println!("\t({})", flags.join(", "));
                }
            })
        };
    print_versions(available_versions, &mut installed_versions, true);
    print_versions(installed_versions.clone(), &mut installed_versions, false);

    Ok(())
}

pub fn get_latest_version() -> Result<Version> {
    fetch_versions()?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("First version not found"))
}

/// Read the installed anchor-cli versions by reading the binaries in the AVM_HOME/bin directory.
pub fn read_installed_versions() -> Result<Vec<Version>> {
    const PREFIX: &str = "anchor-";
    let versions = fs::read_dir(get_bin_dir_path())?
        .filter_map(|entry_result| entry_result.ok())
        .filter_map(|entry| entry.file_name().to_str().map(|f| f.to_owned()))
        .filter(|file_name| file_name.starts_with(PREFIX))
        .filter_map(|file_name| file_name.trim_start_matches(PREFIX).parse::<Version>().ok())
        .collect();

    Ok(versions)
}

#[cfg(test)]
mod tests {
    use crate::*;
    use semver::Version;
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    #[test]
    fn test_ensure_paths() {
        ensure_paths();
        assert!(AVM_HOME.exists());
        let bin_dir = get_bin_dir_path();
        assert!(bin_dir.exists());
        let current_version_file = current_version_file_path();
        assert!(current_version_file.exists());
    }

    #[test]
    fn test_version_binary_path() {
        assert_eq!(
            version_binary_path(&Version::parse("0.18.2").unwrap()),
            get_bin_dir_path().join("anchor-0.18.2")
        );
    }

    #[test]
    fn test_read_anchorversion() -> Result<()> {
        ensure_paths();

        let anchorversion_path = Path::new(".anchorversion");
        let test_version = "0.26.0";
        fs::write(anchorversion_path, test_version)?;

        let version = read_anchorversion_file()?;
        assert_eq!(version.to_string(), test_version);

        fs::remove_file(anchorversion_path)?;

        Ok(())
    }

    #[test]
    fn test_current_version() {
        ensure_paths();
        let mut current_version_file = fs::File::create(current_version_file_path()).unwrap();
        current_version_file.write_all("0.18.2".as_bytes()).unwrap();
        // Sync the file to disk before the read in current_version() to
        // mitigate the read not seeing the written version bytes.
        current_version_file.sync_all().unwrap();
        assert_eq!(
            current_version().unwrap(),
            Version::parse("0.18.2").unwrap()
        );
    }

    #[test]
    #[should_panic(expected = "anchor-cli 0.18.1 is not installed")]
    fn test_uninstall_non_installed_version() {
        uninstall_version(&Version::parse("0.18.1").unwrap()).unwrap();
    }

    #[test]
    #[should_panic(expected = "anchor-cli 0.18.2 is currently in use")]
    fn test_uninstalled_in_use_version() {
        ensure_paths();
        let version = Version::parse("0.18.2").unwrap();
        let mut current_version_file = fs::File::create(current_version_file_path()).unwrap();
        current_version_file.write_all("0.18.2".as_bytes()).unwrap();
        // Sync the file to disk before the read in current_version() to
        // mitigate the read not seeing the written version bytes.
        current_version_file.sync_all().unwrap();
        // Create a fake binary for anchor-0.18.2 in the bin directory
        fs::File::create(version_binary_path(&version)).unwrap();
        uninstall_version(&version).unwrap();
    }

    #[test]
    fn test_read_installed_versions() {
        ensure_paths();
        let version = Version::parse("0.18.2").unwrap();

        // Create a fake binary for anchor-0.18.2 in the bin directory
        fs::File::create(version_binary_path(&version)).unwrap();
        let expected = vec![version];
        assert_eq!(read_installed_versions().unwrap(), expected);

        // Should ignore this file because its not anchor- prefixed
        fs::File::create(AVM_HOME.join("bin").join("garbage").as_path()).unwrap();
        assert_eq!(read_installed_versions().unwrap(), expected);
    }

    #[test]
    fn test_get_anchor_version_from_commit() {
        let version =
            get_anchor_version_from_commit("e1afcbf71e0f2e10fae14525934a6a68479167b9").unwrap();
        assert_eq!(
            version.to_string(),
            "0.28.0-e1afcbf71e0f2e10fae14525934a6a68479167b9"
        )
    }

    #[test]
    fn test_check_and_get_full_commit_when_full_commit() {
        assert_eq!(
            check_and_get_full_commit("e1afcbf71e0f2e10fae14525934a6a68479167b9").unwrap(),
            "e1afcbf71e0f2e10fae14525934a6a68479167b9"
        )
    }

    #[test]
    fn test_check_and_get_full_commit_when_partial_commit() {
        assert_eq!(
            check_and_get_full_commit("e1afcbf").unwrap(),
            "e1afcbf71e0f2e10fae14525934a6a68479167b9"
        )
    }
}
