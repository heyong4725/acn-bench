//! Tests for `cargo xtask env-hash` (CON-7): the hash over the frozen set, its
//! recorded value, and the `--check` gate.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // CON-19: tests are exempt

mod common;

use std::fs;
use std::path::Path;

use common::{repo_root, strings, xtask_at};

const FROZEN_FILES: &[&str] = &[
    "hypotheses/p4.toml",
    "scenarios/measured/walk/trace.parquet",
    "crates/acn-hyp/src/lib.rs",
    "crates/acn-attrib/src/core/mod.rs",
    "crates/acn-trace/src/schema/mod.rs",
];

const OUTSIDE_FILES: &[&str] = &[
    "crates/acn-trace/src/lib.rs",
    "crates/acn-attrib/src/lib.rs",
    "scenarios/synthetic/a.toml",
    "lab/hypotheses/p17-a2a.toml",
    "specs/000-constitution.md",
    "scenarios/measured/.gitkeep",
];

fn write(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
    fs::write(p, body).expect("write");
}

fn frozen_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for f in FROZEN_FILES.iter().chain(OUTSIDE_FILES) {
        write(dir.path(), f, &format!("content of {f}\n"));
    }
    dir
}

fn hash_of(root: &Path) -> String {
    let run = xtask_at(root, &["env-hash"]);
    assert!(run.ok(), "{}", run.json);
    run.json["env_hash"].as_str().expect("env_hash").to_owned()
}

/// Cites: CON-7
#[test]
fn hash_covers_exactly_the_frozen_set() {
    let dir = frozen_fixture();
    let run = xtask_at(dir.path(), &["env-hash"]);
    assert!(run.ok(), "{}", run.json);
    let mut files: Vec<String> = run.json["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|f| f["path"].as_str().expect("path").to_owned())
        .collect();
    files.sort();
    let mut expected: Vec<String> = FROZEN_FILES.iter().map(|s| (*s).to_owned()).collect();
    expected.sort();
    assert_eq!(files, expected);
}

/// Cites: CON-7
#[test]
fn hash_is_deterministic_and_hex_blake3() {
    let dir = frozen_fixture();
    let a = hash_of(dir.path());
    let b = hash_of(dir.path());
    assert_eq!(a, b);
    assert_eq!(a.len(), 64, "blake3 hex is 64 chars: {a}");
    assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
}

/// Cites: CON-7
#[test]
fn hash_changes_only_when_a_frozen_file_changes() {
    let dir = frozen_fixture();
    let base = hash_of(dir.path());
    for f in OUTSIDE_FILES {
        write(dir.path(), f, "changed\n");
    }
    assert_eq!(
        hash_of(dir.path()),
        base,
        "non-frozen edits must not move the hash"
    );
    write(dir.path(), "hypotheses/p4.toml", "changed\n");
    let moved = hash_of(dir.path());
    assert_ne!(moved, base);
    // A new file inside a frozen directory also moves it.
    write(dir.path(), "hypotheses/p99.toml", "new\n");
    assert_ne!(hash_of(dir.path()), moved);
}

/// Cites: CON-7
#[test]
fn check_fails_without_a_record_and_passes_after_write() {
    let dir = frozen_fixture();
    let no_record = xtask_at(dir.path(), &["env-hash", "--check"]);
    assert!(!no_record.ok(), "{}", no_record.json);

    let written = xtask_at(dir.path(), &["env-hash", "--write"]);
    assert!(written.ok(), "{}", written.json);
    assert!(dir.path().join("env-hash.json").is_file());

    let check = xtask_at(dir.path(), &["env-hash", "--check"]);
    assert!(check.ok(), "{}", check.json);
    assert_eq!(check.json["env_hash"], check.json["recorded"]);

    write(dir.path(), "crates/acn-hyp/src/lib.rs", "tampered\n");
    let tampered = xtask_at(dir.path(), &["env-hash", "--check"]);
    assert!(!tampered.ok(), "{}", tampered.json);
    assert_ne!(tampered.json["env_hash"], tampered.json["recorded"]);
}

/// Cites: CON-7
#[test]
fn self_host_the_recorded_hash_matches_the_workspace() {
    let run = xtask_at(&repo_root(), &["env-hash", "--check"]);
    assert!(run.ok(), "{}", run.json);
    let files = strings(&run.json, "files");
    assert!(
        files.iter().any(|f| f.contains("hypotheses/p4.toml")),
        "hypotheses/p4.toml must be in the frozen set: {files:?}"
    );
}

/// Cites: CON-8
#[test]
fn env_hash_honours_the_json_contract_on_both_outcomes() {
    let dir = frozen_fixture();
    assert_eq!(xtask_at(dir.path(), &["env-hash", "--check"]).code, Some(1));
    assert_eq!(xtask_at(dir.path(), &["env-hash", "--write"]).code, Some(0));
    assert_eq!(xtask_at(dir.path(), &["env-hash", "--check"]).code, Some(0));
}
