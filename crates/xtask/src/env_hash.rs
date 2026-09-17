//! `cargo xtask env-hash` (CON-7): a blake3 over the frozen set, recorded in
//! `env-hash.json` at the workspace root and checked as a gate (CON-9). See ADR-4.

use std::path::Path;

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::workspace::{read, read_bytes, rel, write};
use crate::{Error, Result};

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

/// Per-file difference between the computed and the recorded set.
#[derive(Debug, Default, Serialize)]
pub struct Diff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
}

/// The JSON object `env-hash` prints (CON-8).
#[derive(Debug, Serialize)]
pub struct Report {
    pub ok: bool,
    pub mode: &'static str,
    pub env_hash: String,
    pub recorded: Option<String>,
    pub record_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<Diff>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub files: Vec<FileHash>,
}

fn is_placeholder(name: &str) -> bool {
    name == ".gitkeep" || name == ".DS_Store"
}

/// Compute the hash over the frozen set under `root`. Symlinks anywhere in
/// the frozen set are refused: their targets would be hashed by name only.
pub fn compute(root: &Path) -> Result<EnvHash> {
    let mut files = Vec::new();
    for base in FROZEN_SET {
        let dir = root.join(base);
        if !dir.exists() {
            continue;
        }
        for entry in WalkDir::new(&dir).sort_by_file_name() {
            let entry = entry?;
            if entry.path_is_symlink() {
                return Err(Error::Invalid(format!(
                    "symlink inside the frozen set is not allowed: {}",
                    rel(root, entry.path())
                )));
            }
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
    let parsed: EnvHash = serde_json::from_str(&read(&path)?).map_err(|e| {
        Error::Invalid(format!(
            "{RECORD_FILE} is not a valid env-hash record ({e}); regenerate it with `cargo xtask env-hash --write` in an `env-change` PR"
        ))
    })?;
    Ok(Some(parsed))
}

fn diff(computed: &EnvHash, recorded: &EnvHash) -> Diff {
    let mut d = Diff::default();
    for f in &computed.files {
        match recorded.files.iter().find(|r| r.path == f.path) {
            None => d.added.push(f.path.clone()),
            Some(r) if r.blake3 != f.blake3 => d.changed.push(f.path.clone()),
            Some(_) => {}
        }
    }
    for r in &recorded.files {
        if !computed.files.iter().any(|f| f.path == r.path) {
            d.removed.push(r.path.clone());
        }
    }
    d
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
    let mut report = Report {
        ok: true,
        mode: match mode {
            Mode::Print => "print",
            Mode::Check => "check",
            Mode::Write => "write",
        },
        env_hash: computed.env_hash.clone(),
        recorded: recorded_hash.clone(),
        record_file: RECORD_FILE.to_owned(),
        diff: None,
        error: None,
        files: computed.files.clone(),
    };
    match mode {
        Mode::Print => {}
        Mode::Check => {
            let matches = recorded_hash.as_deref() == Some(computed.env_hash.as_str());
            if !matches {
                report.ok = false;
                report.diff = rec.as_ref().map(|r| diff(&computed, r));
                let hint = if rec.is_some() {
                    "frozen set differs from env-hash.json: if the change is intended, run `cargo xtask env-hash --write` and label the PR `env-change` (CON-7)"
                } else {
                    "env-hash.json is missing: run `cargo xtask env-hash --write`"
                };
                tracing::error!(
                    computed = %computed.env_hash,
                    recorded = recorded_hash.as_deref().unwrap_or("<none>"),
                    "{hint}"
                );
                report.error = Some(hint.to_owned());
            }
        }
        Mode::Write => {
            let mut text = serde_json::to_string_pretty(&computed)?;
            text.push('\n');
            write(&root.join(RECORD_FILE), &text)?;
        }
    }
    Ok(report)
}
