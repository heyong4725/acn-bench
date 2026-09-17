//! Tests for `cargo xtask docs-inventory` (gate step in CON-9).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // CON-19: tests are exempt

mod common;

use std::fs;

use common::{fixture, repo_root, strings, xtask_at};

fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
    for entry in walkdir::WalkDir::new(from) {
        let entry = entry.expect("walk");
        let rel = entry.path().strip_prefix(from).expect("prefix");
        let dest = to.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&dest).expect("mkdir");
        } else {
            fs::copy(entry.path(), &dest).expect("copy");
        }
    }
}

/// Cites: CON-9
#[test]
fn check_fails_until_generated_docs_are_written_then_passes() {
    let dir = tempfile::tempdir().expect("tempdir");
    copy_dir(&fixture("ok"), dir.path());

    let before = xtask_at(dir.path(), &["docs-inventory", "--check"]);
    assert!(!before.ok(), "{}", before.json);
    assert!(!strings(&before.json, "changed").is_empty());

    let written = xtask_at(dir.path(), &["docs-inventory"]);
    assert!(written.ok(), "{}", written.json);
    let files = strings(&written.json, "written");
    assert!(
        files
            .iter()
            .any(|f| f.ends_with("docs/generated/requirements.md")),
        "{files:?}"
    );
    let req = fs::read_to_string(dir.path().join("docs/generated/requirements.md")).expect("read");
    assert!(req.contains("FIX-1"), "{req}");
    assert!(req.contains("fixture_is_cited"), "{req}");

    let after = xtask_at(dir.path(), &["docs-inventory", "--check"]);
    assert!(after.ok(), "{}", after.json);

    fs::write(dir.path().join("docs/generated/requirements.md"), "stale\n").expect("write");
    let stale = xtask_at(dir.path(), &["docs-inventory", "--check"]);
    assert!(!stale.ok(), "{}", stale.json);
}

/// Cites: CON-9
#[test]
fn generation_is_deterministic() {
    let dir = tempfile::tempdir().expect("tempdir");
    copy_dir(&fixture("ok"), dir.path());
    assert!(xtask_at(dir.path(), &["docs-inventory"]).ok());
    let first =
        fs::read_to_string(dir.path().join("docs/generated/requirements.md")).expect("read");
    assert!(xtask_at(dir.path(), &["docs-inventory"]).ok());
    let second =
        fs::read_to_string(dir.path().join("docs/generated/requirements.md")).expect("read");
    assert_eq!(first, second);
}

/// Cites: CON-9
#[test]
fn self_host_generated_docs_are_current() {
    let run = xtask_at(&repo_root(), &["docs-inventory", "--check"]);
    assert!(run.ok(), "{}", run.json);
}
