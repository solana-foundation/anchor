//! Build-time support for linking hand-written SBPF assembly into Anchor v2
//! programs via `global_asm!`.
//!
//! Two usage modes:
//!
//! ## Simple mode — port existing assembly projects
//!
//! For projects that already have `.s` files with `.equ` constants (e.g.
//! from an external injection system), just concatenate and link:
//!
//! ```toml
//! # Cargo.toml
//! [build-dependencies]
//! anchor-asm-v2 = { path = "..." }
//! ```
//!
//! ```rust,ignore
//! // build.rs
//! fn main() {
//!     anchor_asm_v2::build("src/asm");
//! }
//! ```
//!
//! ```rust,ignore
//! // lib.rs
//! #![no_std]
//! #![no_main]
//! #![feature(asm_experimental_arch)]
//!
//! anchor_asm_v2::include_asm!();
//!
//! #[panic_handler]
//! fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }
//! ```
//!
//! `build()` walks the assembly directory, expands `.include` directives,
//! and writes a single `$OUT_DIR/combined.s`. `include_asm!()` wraps it
//! in `global_asm!`.
//!
//! ## Full mode — new programs with compile-time constants
//!
//! For new assembly programs, use `asm_program!` from the companion
//! `anchor-asm-v2-macros` crate to define types and generate `const`
//! operands in one block:
//!
//! ```rust,ignore
//! anchor_asm_v2_macros::asm_program! {
//!     #[error_enum(prefix = "E")]
//!     pub enum ErrorCode { InvalidDiscriminant, ... }
//!
//!     #[frame(prefix = "FM")]
//!     #[repr(C)]
//!     pub struct Frame { pub saved_r6: u64, pub bump: u8, ... }
//!
//!     asm { include_str!(concat!(env!("OUT_DIR"), "/combined.s")), }
//! }
//! ```

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Simple mode: build() + include_asm!()
// ---------------------------------------------------------------------------

/// Emit `global_asm!` linking the combined assembly from `build()`.
///
/// Call at crate root scope. Requires `#![feature(asm_experimental_arch)]`.
#[macro_export]
macro_rules! include_asm {
    () => {
        core::arch::global_asm!(include_str!(concat!(env!("OUT_DIR"), "/combined.s")));
    };
}

/// Build-time entry point. Call from `build.rs` with the path to the
/// assembly source directory (relative to the crate root).
///
/// Walks the directory for `.s` files, expands `.include` directives,
/// and writes the concatenated result to `$OUT_DIR/combined.s`.
///
/// If `src/lib.rs` contains `#[repr(C)]` or `#[account]` structs,
/// `.equ` constants for field offsets are prepended automatically.
pub fn build(asm_dir: &str) {
    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"),
    );
    let out_dir = PathBuf::from(
        std::env::var("OUT_DIR").expect("OUT_DIR not set"),
    );
    let asm_path = manifest_dir.join(asm_dir);
    let lib_rs = manifest_dir.join("src").join("lib.rs");

    let preamble = if lib_rs.exists() {
        preamble::generate(&lib_rs)
    } else {
        String::new()
    };

    let combined = collect_asm(&asm_path);

    let output = if preamble.is_empty() {
        combined
    } else {
        format!("{preamble}\n{combined}")
    };

    std::fs::write(out_dir.join("combined.s"), output).expect("write combined.s");

    println!("cargo:rerun-if-changed={asm_dir}");
    if lib_rs.exists() {
        println!("cargo:rerun-if-changed=src/lib.rs");
    }
}

/// Like `build()` but takes absolute paths. Skips the preamble — the
/// caller handles any preprocessing.
pub fn build_to(asm_dir: &Path, output_path: &Path) {
    let combined = collect_asm(asm_dir);
    std::fs::write(output_path, combined).expect("write combined assembly");
    println!("cargo:rerun-if-changed={}", asm_dir.display());
}

// ---------------------------------------------------------------------------
// Assembly collection and .include expansion
// ---------------------------------------------------------------------------

mod preamble;

fn collect_asm(dir: &Path) -> String {
    let mut files: Vec<PathBuf> = Vec::new();
    walk_dir(dir, &mut files);
    files.sort();

    let root_file = find_root_file(dir, &files);

    if let Some(root) = root_file {
        expand_includes(&root, dir)
    } else {
        let mut out = String::new();
        for file in &files {
            let content = std::fs::read_to_string(file)
                .unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
            out.push_str(&format!(
                "# --- {} ---\n",
                file.strip_prefix(dir).unwrap_or(file).display()
            ));
            out.push_str(&content);
            out.push('\n');
        }
        out
    }
}

/// Find the root assembly file. Priority: dir-name match (e.g.
/// `dropset/dropset.s`) first, then well-known names.
fn find_root_file(dir: &Path, files: &[PathBuf]) -> Option<PathBuf> {
    if let Some(dir_name) = dir.file_name().and_then(|n| n.to_str()) {
        let candidate = dir.join(format!("{dir_name}.s"));
        if files.contains(&candidate) {
            return Some(candidate);
        }
    }

    for name in ["entrypoint.s", "main.s"] {
        let candidate = dir.join(name);
        if files.contains(&candidate) {
            return Some(candidate);
        }
    }

    None
}

fn expand_includes(path: &Path, base_dir: &Path) -> String {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

    let mut out = String::new();
    let rel = path.strip_prefix(base_dir).unwrap_or(path);
    out.push_str(&format!("# --- {} ---\n", rel.display()));

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(".include") {
            let file = rest.trim().trim_matches('"');
            let include_path = path.parent().unwrap_or(base_dir).join(file);
            if include_path.exists() {
                out.push_str(&expand_includes(&include_path, base_dir));
            } else {
                let from_base = base_dir.join(file);
                if from_base.exists() {
                    out.push_str(&expand_includes(&from_base, base_dir));
                } else {
                    out.push_str(line);
                    out.push('\n');
                }
            }
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn walk_dir(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("s") {
            out.push(path);
        }
    }
}
