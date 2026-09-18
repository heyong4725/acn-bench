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

/// Cites: CON-9
#[test]
fn stale_files_fail_the_check_and_a_write_removes_them() {
    let dir = tempfile::tempdir().expect("tempdir");
    copy_dir(&fixture("ok"), dir.path());
    assert!(xtask_at(dir.path(), &["docs-inventory"]).ok());
    for stale in ["old.md", ".hidden.md"] {
        fs::write(dir.path().join("docs/generated").join(stale), "x\n").expect("write");
    }
    let check = xtask_at(dir.path(), &["docs-inventory", "--check"]);
    assert!(!check.ok(), "{}", check.json);
    assert_eq!(
        strings(&check.json, "stale"),
        vec!["docs/generated/.hidden.md", "docs/generated/old.md"]
    );
    let write = xtask_at(dir.path(), &["docs-inventory"]);
    assert!(write.ok(), "{}", write.json);
    assert_eq!(strings(&write.json, "removed").len(), 2, "{}", write.json);
    assert!(
        xtask_at(dir.path(), &["docs-inventory", "--check"]).ok(),
        "a write must converge"
    );
}

/// Cites: CON-9
#[test]
fn decision_records_are_ordered_numerically_with_a_name_tie_break() {
    let dir = tempfile::tempdir().expect("tempdir");
    copy_dir(&fixture("ok"), dir.path());
    let adr = dir.path().join("docs/decisions");
    fs::create_dir_all(&adr).expect("mkdir");
    for (name, title) in [
        ("ADR-10.md", "Ten"),
        ("ADR-9.md", "Nine | piped"),
        ("ADR-zeta.md", "Zeta"),
        ("ADR-alpha.md", "Alpha"),
    ] {
        fs::write(
            adr.join(name),
            format!(
                "# {} — {title}\n\n**Status:** accepted. **IDs affected:** FIX-1.\n",
                name.trim_end_matches(".md")
            ),
        )
        .expect("write");
    }
    assert!(xtask_at(dir.path(), &["docs-inventory"]).ok());
    let md = fs::read_to_string(dir.path().join("docs/generated/decisions.md")).expect("read");
    let order: Vec<usize> = ["ADR-9]", "ADR-10]", "ADR-alpha]", "ADR-zeta]"]
        .iter()
        .map(|n| md.find(n).expect(n))
        .collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{md}");
    assert!(md.contains("Nine \\| piped"), "pipes are escaped: {md}");
    assert!(md.contains("| accepted | FIX-1 |"), "{md}");
}

/// Cites: CON-9
#[cfg(unix)]
#[test]
fn a_symlinked_output_is_refused_so_nothing_outside_the_directory_is_overwritten() {
    let dir = tempfile::tempdir().expect("tempdir");
    copy_dir(&fixture("ok"), dir.path());
    fs::create_dir_all(dir.path().join("docs/generated")).expect("mkdir");
    fs::write(dir.path().join("victim.txt"), "precious\n").expect("write");
    std::os::unix::fs::symlink(
        dir.path().join("victim.txt"),
        dir.path().join("docs/generated/requirements.md"),
    )
    .expect("symlink");
    let run = xtask_at(dir.path(), &["docs-inventory"]);
    assert!(!run.ok(), "{}", run.json);
    assert_eq!(
        fs::read_to_string(dir.path().join("victim.txt")).expect("read"),
        "precious\n"
    );
}
