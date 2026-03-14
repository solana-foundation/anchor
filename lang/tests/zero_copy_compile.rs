use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_test_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    env::temp_dir().join(format!("anchor-{name}-{unique}"))
}

#[test]
fn zero_copy_rejects_host_sized_ints() {
    let dir = temp_test_dir("zero-copy-host-sized");
    fs::create_dir_all(dir.join("src")).unwrap();

    fs::write(
        dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "zero_copy_host_sized"
version = "0.1.0"
edition = "2021"

[dependencies]
anchor-lang = {{ path = "{}" }}
bytemuck = {{ version = "1", features = ["derive", "min_const_generics"] }}
"#,
            env!("CARGO_MANIFEST_DIR")
        ),
    )
    .unwrap();

    fs::write(
        dir.join("src/lib.rs"),
        r#"use anchor_lang::prelude::*;

declare_id!("11111111111111111111111111111111");

#[account(zero_copy)]
pub struct HostSized {
    pub value: usize,
}
"#,
    )
    .unwrap();

    let output = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(dir.join("Cargo.toml"))
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "cargo check unexpectedly succeeded"
    );
    assert!(
        stderr.contains("safe zero_copy does not support `usize` or `isize`"),
        "unexpected stderr:\n{stderr}"
    );

    let _ = fs::remove_dir_all(dir);
}
