use anyhow::{Context, Result};
use gix::ThreadSafeRepository;
use std::path::Path;

pub fn init_repository(path: &Path) -> Result<ThreadSafeRepository> {
    gix::init(path)
        .map(|r| r.into_sync())
        .context("Failed to initialize git repository")
}

pub fn has_git_repository(path: &Path) -> bool {
    gix::open(path).is_ok()
}
