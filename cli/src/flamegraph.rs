mod svg;
mod walker;

use crate::{
    build,
    config::{Config, ConfigOverride},
    BootstrapMode,
};
use anstyle_hyperlink::Hyperlink;
use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Builds the selected programs and emits one SVG flamegraph per program.
pub fn flamegraph(
    cfg_override: &ConfigOverride,
    program_name: Option<String>,
    open: bool,
) -> Result<()> {
    let selected_program = program_name.clone();

    build(
        cfg_override,
        true,
        None,
        None,
        false,
        true,
        true,
        program_name,
        None,
        None,
        BootstrapMode::None,
        None,
        None,
        Vec::new(),
        vec!["--debug".into()],
        false,
    )?;

    let cfg = Config::discover(cfg_override)?
        .ok_or_else(|| anyhow!("The 'anchor flamegraph' command requires an Anchor workspace."))?;
    let workspace_root = cfg
        .path()
        .parent()
        .ok_or_else(|| anyhow!("Invalid Anchor.toml"))?
        .canonicalize()?;
    let flamegraph_dir = workspace_root.join("target").join("flamegraph");
    fs::create_dir_all(&flamegraph_dir)?;
    let mut generated_paths = Vec::new();

    for program in cfg.get_programs(selected_program)? {
        let Some(report) = walker::analyze_program(&program)? else {
            continue;
        };
        let svg_path = flamegraph_dir.join(format!("{}.svg", program.lib_name));
        fs::write(&svg_path, svg::render(&report))?;
        let canonical_svg_path = svg_path.canonicalize()?;
        let hyperlink = Hyperlink::with_path(&canonical_svg_path);
        println!(
            "{}: {hyperlink}{}{hyperlink:#}",
            program.lib_name,
            canonical_svg_path.display()
        );
        generated_paths.push(canonical_svg_path);
    }

    if open {
        for svg_path in &generated_paths {
            open_in_browser(svg_path)?;
        }
    }

    Ok(())
}

/// Opens a generated flamegraph SVG in the system browser.
fn open_in_browser(path: &Path) -> Result<()> {
    let target = anstyle_hyperlink::file_to_url(None, path)
        .ok_or_else(|| anyhow!("Failed to create file URL for {}", path.display()))?;
    let mut command = browser_open_command(&target);
    let status = command.status()?;
    if !status.success() {
        return Err(anyhow!(
            "Failed to open flamegraph in browser: {}",
            path.display()
        ));
    }
    Ok(())
}

/// Builds the platform-specific command used to open a browser target.
fn browser_open_command(target: &str) -> Command {
    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("open");
        command.arg(target);
        return command;
    }

    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", target]);
        return command;
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let mut command = Command::new("xdg-open");
        command.arg(target);
        command
    }
}
