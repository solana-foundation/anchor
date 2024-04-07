use std::{
    collections::BTreeMap,
    env, mem,
    path::Path,
    process::{Command, Stdio},
};

use anchor_syn::parser::context::CrateContext;
use anyhow::{anyhow, Result};
use regex::Regex;
use serde::Deserialize;

use crate::types::{Idl, IdlEvent, IdlTypeDef};

/// A trait that types must implement in order to include the type in the IDL definition.
///
/// This trait is automatically implemented for Anchor all types that use the `AnchorSerialize`
/// proc macro. Note that manually implementing the `AnchorSerialize` trait does **NOT** have the
/// same effect.
///
/// Types that don't implement this trait will cause a compile error during the IDL generation.
///
/// The default implementation of the trait allows the program to compile but the type does **NOT**
/// get included in the IDL.
pub trait IdlBuild {
    /// Create an IDL type definition for the type.
    ///
    /// The type is only included in the IDL if this method returns `Some`.
    fn create_type() -> Option<IdlTypeDef> {
        None
    }

    /// Insert all types that are included in the current type definition to the given map.
    fn insert_types(_types: &mut BTreeMap<String, IdlTypeDef>) {}

    /// Get the full module path of the type.
    ///
    /// The full path will be used in the case of a conflicting type definition, e.g. when there
    /// are multiple structs with the same name.
    ///
    /// The default implementation covers most cases.
    fn get_full_path() -> String {
        std::any::type_name::<Self>().into()
    }
}

/// Generate IDL via compilation.
pub fn build_idl(
    program_path: impl AsRef<Path>,
    resolution: bool,
    skip_lint: bool,
    no_docs: bool,
) -> Result<Idl> {
    // Check safety comments
    let program_path = program_path.as_ref();
    let lib_path = program_path.join("src").join("lib.rs");
    let ctx = CrateContext::parse(lib_path)?;
    if !skip_lint {
        ctx.safety_checks()?;
    }

    let idl = build(program_path, resolution, no_docs)?;
    let idl = convert_module_paths(idl);
    let idl = sort(idl);
    verify(&idl)?;

    Ok(idl)
}

/// Build IDL.
fn build(program_path: &Path, resolution: bool, no_docs: bool) -> Result<Idl> {
    // `nightly` toolchain is currently required for building the IDL.
    const TOOLCHAIN: &str = "+nightly";
    install_toolchain_if_needed(TOOLCHAIN)?;

    let output = Command::new("cargo")
        .args([
            TOOLCHAIN,
            "test",
            "__anchor_private_print_idl",
            "--features",
            "idl-build",
            "--",
            "--show-output",
            "--quiet",
        ])
        .env(
            "ANCHOR_IDL_BUILD_NO_DOCS",
            if no_docs { "TRUE" } else { "FALSE" },
        )
        .env(
            "ANCHOR_IDL_BUILD_RESOLUTION",
            if resolution { "TRUE" } else { "FALSE" },
        )
        .env("RUSTFLAGS", "--cfg procmacro2_semver_exempt")
        .current_dir(program_path)
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(anyhow!("Building IDL failed"));
    }

    enum State {
        Pass,
        Address,
        Constants(Vec<String>),
        Events(Vec<String>),
        Errors(Vec<String>),
        Program(Vec<String>),
    }

    let mut address = String::new();
    let mut events = vec![];
    let mut error_codes = vec![];
    let mut constants = vec![];
    let mut types = BTreeMap::new();
    let mut idl: Option<Idl> = None;

    let output = String::from_utf8_lossy(&output.stdout);
    if env::var("ANCHOR_LOG").is_ok() {
        println!("{}", output);
    }

    let mut state = State::Pass;
    for line in output.lines() {
        match &mut state {
            State::Pass => match line {
                "--- IDL begin address ---" => state = State::Address,
                "--- IDL begin const ---" => state = State::Constants(vec![]),
                "--- IDL begin event ---" => state = State::Events(vec![]),
                "--- IDL begin errors ---" => state = State::Errors(vec![]),
                "--- IDL begin program ---" => state = State::Program(vec![]),
                _ => {
                    if line.starts_with("test result: ok") {
                        if let Some(idl) = idl.as_mut() {
                            idl.address = mem::take(&mut address);
                            idl.constants = mem::take(&mut constants);
                            idl.events = mem::take(&mut events);
                            idl.errors = mem::take(&mut error_codes);
                            idl.types = {
                                let prog_ty = mem::take(&mut idl.types);
                                let mut types = mem::take(&mut types);
                                types.extend(prog_ty.into_iter().map(|ty| (ty.name.clone(), ty)));
                                types.into_values().collect()
                            };
                        }
                    }
                }
            },
            State::Address => {
                address = line.replace(|c: char| !c.is_alphanumeric(), "");
                state = State::Pass;
                continue;
            }
            State::Constants(lines) => {
                if line == "--- IDL end const ---" {
                    let constant = serde_json::from_str(&lines.join("\n"))?;
                    constants.push(constant);
                    state = State::Pass;
                    continue;
                }

                lines.push(line.to_owned());
            }
            State::Events(lines) => {
                if line == "--- IDL end event ---" {
                    #[derive(Deserialize)]
                    struct IdlBuildEventPrint {
                        event: IdlEvent,
                        types: Vec<IdlTypeDef>,
                    }

                    let event = serde_json::from_str::<IdlBuildEventPrint>(&lines.join("\n"))?;
                    events.push(event.event);
                    types.extend(event.types.into_iter().map(|ty| (ty.name.clone(), ty)));
                    state = State::Pass;
                    continue;
                }

                lines.push(line.to_owned());
            }
            State::Errors(lines) => {
                if line == "--- IDL end errors ---" {
                    error_codes = serde_json::from_str(&lines.join("\n"))?;
                    state = State::Pass;
                    continue;
                }

                lines.push(line.to_owned());
            }
            State::Program(lines) => {
                if line == "--- IDL end program ---" {
                    idl = Some(serde_json::from_str(&lines.join("\n"))?);
                    state = State::Pass;
                    continue;
                }

                lines.push(line.to_owned());
            }
        }
    }

    idl.ok_or_else(|| anyhow!("IDL doesn't exist"))
}

/// Install the given toolchain if it's not already installed.
fn install_toolchain_if_needed(toolchain: &str) -> Result<()> {
    let is_installed = Command::new("cargo")
        .arg(toolchain)
        .output()?
        .status
        .success();
    if !is_installed {
        Command::new("rustup")
            .args(["toolchain", "install", toolchain.trim_start_matches('+')])
            .spawn()?
            .wait()?;
    }

    Ok(())
}

/// Convert paths to name if there are no conflicts.
fn convert_module_paths(idl: Idl) -> Idl {
    let idl = serde_json::to_string(&idl).unwrap();
    let idl = Regex::new(r#""((\w+::)+)(\w+)""#)
        .unwrap()
        .captures_iter(&idl.clone())
        .fold(idl, |acc, cur| {
            let path = cur.get(0).unwrap().as_str();
            let name = cur.get(3).unwrap().as_str();

            // Replace path with name
            let replaced_idl = acc.replace(path, &format!(r#""{name}""#));

            // Check whether there is a conflict
            let has_conflict = replaced_idl.contains(&format!(r#"::{name}""#));
            if has_conflict {
                acc
            } else {
                replaced_idl
            }
        });

    serde_json::from_str(&idl).expect("Invalid IDL")
}

/// Alphabetically sort fields for consistency.
fn sort(mut idl: Idl) -> Idl {
    idl.accounts.sort_by(|a, b| a.name.cmp(&b.name));
    idl.constants.sort_by(|a, b| a.name.cmp(&b.name));
    idl.events.sort_by(|a, b| a.name.cmp(&b.name));
    idl.instructions.sort_by(|a, b| a.name.cmp(&b.name));
    idl.types.sort_by(|a, b| a.name.cmp(&b.name));

    idl
}

/// Verify IDL is valid.
fn verify(idl: &Idl) -> Result<()> {
    // Check full path accounts
    if let Some(account) = idl
        .accounts
        .iter()
        .find(|account| account.name.contains("::"))
    {
        return Err(anyhow!(
            "Conflicting accounts names are not allowed.\nProgram: `{}`\nAccount: `{}`",
            idl.metadata.name,
            account.name
        ));
    }

    Ok(())
}
