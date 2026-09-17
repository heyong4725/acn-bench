//! Workspace-root helpers shared by every task.

use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// The workspace root when none is given: two levels above this crate.
pub fn default_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .map_or_else(|| manifest.to_path_buf(), Path::to_path_buf)
}

/// Read a file to a string, attaching the path to any error.
pub fn read(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|e| Error::io(path, e))
}

/// Read a file's bytes, attaching the path to any error.
pub fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|e| Error::io(path, e))
}

/// Write a file, creating parent directories, attaching the path to any error.
pub fn write(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    std::fs::write(path, contents).map_err(|e| Error::io(path, e))
}

/// A path relative to `root`, with forward slashes, for stable output on every platform.
pub fn rel(root: &Path, path: &Path) -> String {
    let p = path.strip_prefix(root).unwrap_or(path);
    p.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Directory names never scanned for sources or specs.
pub fn is_skipped_dir(name: &str) -> bool {
    name == "target" || name == "fixtures" || name.starts_with('.')
}
