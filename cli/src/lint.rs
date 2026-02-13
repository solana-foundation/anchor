use crate::config::{Config, ConfigOverride};
use anyhow::{anyhow, Result};
use std::fs;
use walkdir::WalkDir;
use regex::Regex;

pub fn lint(cfg_override: &ConfigOverride, program_name: Option<String>) -> Result<()> {
    println!("🔍 Starting Security Linting...");

    let cfg = Config::discover(cfg_override)?
        .ok_or_else(|| anyhow!("The 'anchor lint' command requires an Anchor workspace."))?;

    let program_list = cfg.get_rust_program_list()?;
    
    for program_path in program_list {
        let name = program_path.file_name().unwrap().to_str().unwrap();
        if let Some(ref target) = program_name {
            if name != target { continue; }
        }

        println!("\n🛡️  Linting program: \x1b[1;36m{}\x1b[0m", name);
        scan_program_for_vulnerabilities(&program_path)?;
    }

    println!("\n✅ Linting complete.");
    Ok(())
}

fn scan_program_for_vulnerabilities(path: &std::path::Path) -> Result<()> {
    // Regex for common vulnerabilities
    let re_entrancy = Regex::new(r"invoke\(")?; // CPI calls without proper checks
    let ownership_check = Regex::new(r"Account < 'info , ([\w]+) >")?; // Basic account usage
    let constraint_mut = Regex::new(r"#\[account\(mut")?;

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().map_or(false, |ext| ext == "rust" || ext == "rs") {
            let content = fs::read_to_string(entry.path())?;
            let filename = entry.path().file_name().unwrap().to_str().unwrap();

            // Rule 1: Re-entrancy check
            if re_entrancy.is_match(&content) {
                println!("   [!] \x1b[1;33mPotential Re-entrancy\x1b[0m in {}: CPI 'invoke' detected. Ensure state is updated BEFORE the call.", filename);
            }

            // Rule 2: Account Ownership Check
            if ownership_check.is_match(&content) && !content.contains("has_one") && !content.contains("owner") {
                println!("   [!] \x1b[1;31mMissing Ownership Check\x1b[0m in {}: Found Account usage without 'has_one' or 'owner' constraints on sensitive accounts.", filename);
            }

            // Rule 3: Mutable account without check
            if constraint_mut.is_match(&content) && !content.contains("signer") {
                 println!("   [!] \x1b[1;33mWarning\x1b[0m in {}: Mutable account used without 'signer' constraint. Verify if this is intended.", filename);
            }
        }
    }
    Ok(())
}
