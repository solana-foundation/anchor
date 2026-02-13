use crate::config::{Config, ConfigOverride};
use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;
use serde_json::Value;

pub fn doc_gen(
    cfg_override: &ConfigOverride,
    program_name: Option<String>,
    out: Option<String>,
) -> Result<()> {
    println!("📚 Starting Auto-Documentation Generation...");

    let cfg = Config::discover(cfg_override)?
        .ok_or_else(|| anyhow!("The 'anchor doc' command requires an Anchor workspace."))?;

    let cfg_parent = cfg.path().parent().unwrap();
    let idl_dir = cfg_parent.join("target").join("idl");

    if !idl_dir.exists() {
        return Err(anyhow!("IDL directory not found. Please run 'anchor build' first."));
    }

    let entries = fs::read_dir(idl_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "json") {
            let filename = path.file_stem().unwrap().to_str().unwrap();
            if let Some(ref target) = program_name {
                if filename != target { continue; }
            }

            println!("\n📖 Generating docs for: \x1b[1;35m{}\x1b[0m", filename);
            generate_markdown_from_idl(&path, out.as_ref().map(PathBuf::from))?;
        }
    }

    println!("\n✅ Documentation generated successfully.");
    Ok(())
}

fn generate_markdown_from_idl(idl_path: &PathBuf, out_dir: Option<PathBuf>) -> Result<()> {
    let content = fs::read_to_string(idl_path)?;
    let idl: Value = serde_json::from_str(&content)?;

    let name = idl["metadata"]["name"].as_str().unwrap_or("Unknown Program");
    let mut markdown = format!("# Program Documentation: {}\n\n", name);
    
    markdown.push_str("## Instructions\n\n");
    if let Some(instructions) = idl["instructions"].as_array() {
        for inst in instructions {
            let inst_name = inst["name"].as_str().unwrap_or("unknown");
            markdown.push_str(&format!("### `{}`\n", inst_name));
            markdown.push_str("#### Accounts:\n");
            if let Some(accounts) = inst["accounts"].as_array() {
                for acc in accounts {
                    let acc_name = acc["name"].as_str().unwrap_or("acc");
                    let is_mut = acc["isMut"].as_bool().unwrap_or(false);
                    let is_signer = acc["isSigner"].as_bool().unwrap_or(false);
                    markdown.push_str(&format!("- `{}` (mut: {}, signer: {})\n", acc_name, is_mut, is_signer));
                }
            }
            markdown.push_str("\n");
        }
    }

    markdown.push_str("## Accounts (State)\n\n");
    if let Some(accounts) = idl["accounts"].as_array() {
        for acc in accounts {
            let acc_name = acc["name"].as_str().unwrap_or("unknown");
            markdown.push_str(&format!("### `{}`\n", acc_name));
            // simplified type display
            markdown.push_str("Defined in program state.\n\n");
        }
    }

    let out_path = out_dir.unwrap_or_else(|| idl_path.parent().unwrap().to_path_buf())
        .join(format!("{}_README.md", name));
    
    // Add Anchor Lens (Mermaid Visualizer)
    markdown.push_str("## Anchor Lens (Visualizer)\n\n");
    markdown.push_str("```mermaid\ngraph TD\n");
    if let Some(instructions) = idl["instructions"].as_array() {
        for inst in instructions {
            let inst_name = inst["name"].as_str().unwrap_or("unknown");
            markdown.push_str(&format!("    Instruction_{} --> |calls| Program\n", inst_name));
            if let Some(accounts) = inst["accounts"].as_array() {
                for acc in accounts {
                    let acc_name = acc["name"].as_str().unwrap_or("acc");
                    markdown.push_str(&format!("    Instruction_{} --- Account_{}\n", inst_name, acc_name));
                }
            }
        }
    }
    markdown.push_str("```\n");

    fs::write(&out_path, markdown)?;
    println!("   - Created: \x1b[1;34m{}\x1b[0m", out_path.display());

    Ok(())
}
