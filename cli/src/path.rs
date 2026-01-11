use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn parent_dir(path: &Path) -> Result<&Path> {
    path.parent()
        .ok_or_else(|| crate::error::CliError::InvalidPath(path.to_path_buf()).into())
}

pub fn to_str(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| crate::error::CliError::InvalidPath(path.to_path_buf()).into())
}

pub fn canonicalize_safe(path: &Path) -> Result<PathBuf> {
    path.canonicalize()
        .map_err(|_| crate::error::CliError::InvalidPath(path.to_path_buf()).into())
}
