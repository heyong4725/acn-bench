//! `cargo xtask env-hash` (CON-7): a blake3 over the frozen set, recorded in
//! `env-hash.json` at the workspace root and checked as a gate (CON-9). See ADR-4.

use std::path::Path;

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::Result;
use crate::workspace::{read, read_bytes, rel, write};

/// The frozen set (CON-7), relative to the workspace root.
pub const FROZEN_SET: &[&str] = &[
    "hypotheses",
    "scenarios/measured",
    "crates/acn-hyp",
    "crates/acn-attrib/src/core",
    "crates/acn-trace/src/schema",
];

/// Where the recorded hash lives, relative to the workspace root.
pub const RECORD_FILE: &str = "env-hash.json";

/// One hashed file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileHash {
    pub path: String,
    pub blake3: String,
}

/// The recorded (and computed) environment hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvHash {
    pub env_hash: String,
    pub files: Vec<FileHash>,
}

/// The JSON object `env-hash` prints (CON-8).
#[derive(Debug, Serialize)]
pub struct Report {
    pub ok: bool,
    pub mode: &'static str,
    pub env_hash: String,
    pub recorded: Option<String>,
    pub record_file: String,
    pub files: Vec<FileHash>,
}

fn is_placeholder(name: &str) -> bool {
    name == ".gitkeep" || name == ".DS_Store"
}

/// Compute the hash over the frozen set under `root`.
pub fn compute(root: &Path) -> Result<EnvHash> {
    let mut files = Vec::new();
    for base in FROZEN_SET {
        let dir = root.join(base);
        if !dir.exists() {
            continue;
        }
        let walker = WalkDir::new(&dir)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| {
                !(e.file_type().is_dir() && e.file_name().to_str().is_some_and(|n| n == "target"))
            });
        for entry in walker {
            let entry = entry?;
            if !entry.file_type().is_file()
                || entry.file_name().to_str().is_some_and(is_placeholder)
            {
                continue;
            }
            let bytes = read_bytes(entry.path())?;
            files.push(FileHash {
                path: rel(root, entry.path()),
                blake3: blake3::hash(&bytes).to_hex().to_string(),
            });
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut hasher = blake3::Hasher::new();
    for f in &files {
        hasher.update(f.path.as_bytes());
        hasher.update(b"\0");
        hasher.update(f.blake3.as_bytes());
        hasher.update(b"\n");
    }
    Ok(EnvHash {
        env_hash: hasher.finalize().to_hex().to_string(),
        files,
    })
}

/// Read the recorded hash, if any.
pub fn recorded(root: &Path) -> Result<Option<EnvHash>> {
    let path = root.join(RECORD_FILE);
    if !path.is_file() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&read(&path)?)?))
}

/// What to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Print,
    Check,
    Write,
}

/// Run in `mode`.
pub fn run(root: &Path, mode: Mode) -> Result<Report> {
    let computed = compute(root)?;
    let rec = recorded(root)?;
    let recorded_hash = rec.as_ref().map(|r| r.env_hash.clone());
    let ok = match mode {
        Mode::Print => true,
        Mode::Check => {
            let matches = recorded_hash.as_deref() == Some(computed.env_hash.as_str());
            if !matches {
                tracing::error!(
                    computed = %computed.env_hash,
                    recorded = recorded_hash.as_deref().unwrap_or("<none>"),
                    "frozen set changed: run `cargo xtask env-hash --write` in an `env-change` PR (CON-7)"
                );
            }
            matches
        }
        Mode::Write => {
            let mut text = serde_json::to_string_pretty(&computed)?;
            text.push('\n');
            write(&root.join(RECORD_FILE), &text)?;
            true
        }
    };
    Ok(Report {
        ok,
        mode: match mode {
            Mode::Print => "print",
            Mode::Check => "check",
            Mode::Write => "write",
        },
        env_hash: computed.env_hash,
        recorded: recorded_hash,
        record_file: RECORD_FILE.to_owned(),
        files: computed.files,
    })
}
