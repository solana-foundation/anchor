use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use reqwest::header::USER_AGENT;
use semver::Version;
use serde::{de, Deserialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;

/// Storage directory for AVM, ~/.avm
pub static AVM_HOME: Lazy<PathBuf> = Lazy::new(|| {
    cfg_if::cfg_if! {
        if #[cfg(test)] {
            let dir = tempfile::tempdir().expect("Could not create temporary directory");
            dir.path().join(".avm")
        } else {
            let mut user_home = dirs::home_dir().expect("Could not find home directory");
            user_home.push(".avm");
            user_home
        }
    }
});

/// Path to the current version file ~/.avm/.version
pub fn current_version_file_path() -> PathBuf {
    let mut current_version_file_path = AVM_HOME.to_path_buf();
    current_version_file_path.push(".version");
    current_version_file_path
}

/// Read the current version from the version file
pub fn current_version() -> Result<Version> {
    let v = fs::read_to_string(current_version_file_path().as_path())
        .map_err(|e| anyhow!("Could not read version file: {}", e))?;
    Version::parse(v.trim_end_matches('\n').to_string().as_str())
        .map_err(|e| anyhow!("Could not parse version file: {}", e))
}

/// Path to the binary for the given version
pub fn version_binary_path(version: &Version) -> PathBuf {
    let mut version_path = AVM_HOME.join("bin");
    version_path.push(format!("anchor-{}", version));
    version_path
}

/// Update the current version to a new version
pub fn use_version(version: &Version) -> Result<()> {
    let installed_versions = read_installed_versions();
    // Make sure the requested version is installed
    if !installed_versions.contains(version) {
        if let Ok(current) = current_version() {
            println!(
                "Version {} is not installed, staying on version {}.",
                version, current
            );
        } else {
            println!("Version {} is not installed, no current version.", version);
        }

        return Err(anyhow!(
            "You need to run 'avm install {}' to install it before using it.",
            version
        ));
    }

    let mut current_version_file = fs::File::create(current_version_file_path().as_path())?;
    current_version_file.write_all(version.to_string().as_bytes())?;
    println!("Now using anchor version {}.", current_version()?);
    Ok(())
}

/// Update to the latest version
pub fn update() -> Result<()> {
    // Find last stable version
    let version = &get_latest_version();

    install_version(version, false)
}

/// Install a version of anchor-cli
pub fn install_version(version: &Version, force: bool) -> Result<()> {
    // If version is already installed we ignore the request.
    let installed_versions = read_installed_versions();
    if installed_versions.contains(version) && !force {
        println!("Version {} is already installed", version);
        return Ok(());
    }

    let exit = std::process::Command::new("cargo")
        .args(&[
            "install",
            "--git",
            "https://github.com/project-serum/anchor",
            "--tag",
            &format!("v{}", &version),
            "anchor-cli",
            "--locked",
            "--root",
            AVM_HOME.to_str().unwrap(),
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| {
            anyhow::format_err!("Cargo install for {} failed: {}", version, e.to_string())
        })?;
    if !exit.status.success() {
        return Err(anyhow!(
            "Failed to install {}, is it a valid version?",
            version
        ));
    }
    fs::rename(
        &AVM_HOME.join("bin").join("anchor"),
        &AVM_HOME.join("bin").join(format!("anchor-{}", version)),
    )?;
    // If .version file is empty or not parseable, write the newly installed version to it
    if current_version().is_err() {
        let mut current_version_file = fs::File::create(current_version_file_path().as_path())?;
        current_version_file.write_all(version.to_string().as_bytes())?;
    }

    use_version(version)
}

/// Remove an installed version of anchor-cli
pub fn uninstall_version(version: &Version) -> Result<()> {
    let version_path = AVM_HOME.join("bin").join(format!("anchor-{}", version));
    if !version_path.exists() {
        return Err(anyhow!("anchor-cli {} is not installed", version));
    }
    if version == &current_version().unwrap() {
        return Err(anyhow!("anchor-cli {} is currently in use", version));
    }
    fs::remove_file(version_path.as_path())?;
    Ok(())
}

/// Ensure the users home directory is setup with the paths required by AVM.
pub fn ensure_paths() {
    let home_dir = AVM_HOME.to_path_buf();
    if !home_dir.as_path().exists() {
        fs::create_dir_all(home_dir.clone()).expect("Could not create .avm directory");
    }
    let bin_dir = home_dir.join("bin");
    if !bin_dir.as_path().exists() {
        fs::create_dir_all(bin_dir).expect("Could not create .avm/bin directory");
    }
    if !current_version_file_path().exists() {
        fs::File::create(current_version_file_path()).expect("Could not create .version file");
    }
}

/// Retrieve a list of installable versions of anchor-cli using the GitHub API and tags on the Anchor
/// repository.
pub fn fetch_versions() -> Vec<semver::Version> {
    #[derive(Deserialize)]
    struct Release {
        #[serde(rename = "name", deserialize_with = "version_deserializer")]
        version: semver::Version,
    }

    fn version_deserializer<'de, D>(deserializer: D) -> Result<semver::Version, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s: &str = de::Deserialize::deserialize(deserializer)?;
        Version::parse(s.trim_start_matches('v')).map_err(de::Error::custom)
    }

    let client = reqwest::blocking::Client::new();
    let versions: Vec<Release> = client
        .get("https://api.github.com/repos/project-serum/anchor/tags")
        .header(USER_AGENT, "avm https://github.com/project-serum/anchor")
        .send()
        .unwrap()
        .json()
        .unwrap();
    versions.into_iter().map(|r| r.version).collect()
}

/// Print available versions and flags indicating installed, current and latest
pub fn list_versions() -> Result<()> {
    let installed_versions = read_installed_versions();

    let mut available_versions = fetch_versions();
    // Reverse version list so latest versions are printed last
    available_versions.reverse();

    available_versions.iter().enumerate().for_each(|(i, v)| {
        print!("{}", v);
        let mut flags = vec![];
        if i == available_versions.len() - 1 {
            flags.push("latest");
        }
        if installed_versions.contains(v) {
            flags.push("installed");
        }
        if current_version().is_ok() && current_version().unwrap() == v.clone() {
            flags.push("current");
        }
        if flags.is_empty() {
            println!();
        } else {
            println!("\t({})", flags.join(", "));
        }
    });

    Ok(())
}

pub fn get_latest_version() -> semver::Version {
    let available_versions = fetch_versions();
    available_versions.first().unwrap().clone()
}

/// Read the installed anchor-cli versions by reading the binaries in the AVM_HOME/bin directory.
pub fn read_installed_versions() -> Vec<semver::Version> {
    let home_dir = AVM_HOME.to_path_buf();
    let mut versions = vec![];
    for file in fs::read_dir(&home_dir.join("bin")).unwrap() {
        let file_name = file.unwrap().file_name();
        // Match only things that look like anchor-*
        if file_name.to_str().unwrap().starts_with("anchor-") {
            let version = file_name
                .to_str()
                .unwrap()
                .trim_start_matches("anchor-")
                .parse::<semver::Version>()
                .unwrap();
            versions.push(version);
        }
    }

    versions
}

#[cfg(test)]
mod tests {
    use crate::*;
    use semver::Version;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_ensure_paths() {
        ensure_paths();
        assert!(AVM_HOME.exists());
        let bin_dir = AVM_HOME.join("bin");
        assert!(bin_dir.exists());
        let current_version_file = AVM_HOME.join(".version");
        assert!(current_version_file.exists());
    }

    #[test]
    fn test_current_version_file_path() {
        ensure_paths();
        assert!(current_version_file_path().exists());
    }

    #[test]
    fn test_version_binary_path() {
        assert!(
            version_binary_path(&Version::parse("0.18.2").unwrap())
                == AVM_HOME.join("bin/anchor-0.18.2")
        );
    }

    #[test]
    fn test_current_version() {
        ensure_paths();
        let mut current_version_file =
            fs::File::create(current_version_file_path().as_path()).unwrap();
        current_version_file.write_all("0.18.2".as_bytes()).unwrap();
        // Sync the file to disk before the read in current_version() to
        // mitigate the read not seeing the written version bytes.
        current_version_file.sync_all().unwrap();
        assert!(current_version().unwrap() == Version::parse("0.18.2").unwrap());
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
        let mut current_version_file =
            fs::File::create(current_version_file_path().as_path()).unwrap();
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
        assert!(read_installed_versions() == expected);
        // Should ignore this file because its not anchor- prefixed
        fs::File::create(AVM_HOME.join("bin").join("garbage").as_path()).unwrap();
        assert!(read_installed_versions() == expected);
    }
}
