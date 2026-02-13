use crate::config::{Config, ConfigOverride};
use anyhow::{anyhow, Result};
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use regex::Regex;

pub fn analyze(
    cfg_override: &ConfigOverride,
    program_name: Option<String>,
    cargo_args: Vec<String>,
) -> Result<()> {
    println!("🚀 Starting Compute Unit Analysis...");

    let _cfg = Config::discover(cfg_override)?
        .ok_or_else(|| anyhow!("The 'anchor analyze' command requires an Anchor workspace."))?;

    // We wrap 'anchor test' to capture logs. 
    // In a real implementation, we would call the internal `test` function with a captured buffer.
    // For this demonstration, we'll simulate the execution of tests and parse the output.

    let mut cmd_args = vec!["test"];
    if let Some(ref name) = program_name {
        cmd_args.push("--program-name");
        cmd_args.push(name);
    }
    for arg in &cargo_args {
        cmd_args.push(arg);
    }

    let mut child = Command::new("anchor")
        .args(&cmd_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);

    let cu_regex = Regex::new(r"consumed (\d+) of (\d+) compute units")?;
    let instruction_regex = Regex::new(r"Program log: Instruction: (\w+)")?;

    let mut current_instruction = String::from("Unknown");

    for line in reader.lines() {
        let line = line?;
        println!("{}", line); // Pass through output

        if let Some(caps) = instruction_regex.captures(&line) {
            current_instruction = caps[1].to_string();
        }

        if let Some(caps) = cu_regex.captures(&line) {
            let consumed: u64 = caps[1].parse()?;
            let total: u64 = caps[2].parse()?;
            
            println!("\n📊 [ANALYSIS] Instruction: \x1b[1;32m{}\x1b[0m", current_instruction);
            println!("   - Consumed: \x1b[1;33m{}\x1b[0m CU", consumed);
            println!("   - Limit: {} CU", total);
            
            if consumed > 150_000 {
                println!("   \x1b[1;31m⚠️  HIGH USAGE DETECTED!\x1b[0m");
                println!("   💡 Suggestion: This instruction is heavy. Consider optimizing loops or using more efficient data structures.");
            } else if consumed > 50_000 {
                println!("   \x1b[1;34mℹ️  MODERATE USAGE\x1b[0m");
                println!("   💡 Suggestion: Check if you are doing redundant lookups in your Loop.");
            }
        }
    }

    child.wait()?;
    println!("\n✅ Analysis complete.");
    Ok(())
}
