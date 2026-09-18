//! Tests for `cargo xtask pr-check`: label rules for specs (CON-14) and the
//! frozen set (CON-7), and CODEOWNERS coverage of protected paths (LOOP-20).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // CON-19: tests are exempt

mod common;

use std::fs;
use std::path::Path;

use common::{repo_root, strings, xtask_at};

const FULL_CODEOWNERS: &str = "\
# owners
/hypotheses/                     @owner
/scenarios/measured/             @owner
/crates/acn-hyp/                 @owner
/crates/acn-attrib/src/core/     @owner
/crates/acn-trace/src/schema/    @owner
/specs/                          @owner
/env-hash.json                   @owner
/trace-scope.toml                @owner
";

fn root_with(codeowners: &str, m0_closed: bool) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), ".github/CODEOWNERS", codeowners);
    if m0_closed {
        write(dir.path(), "docs/gates/M0.md", "# M0\n");
    }
    dir
}

fn write(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
    fs::write(p, body).expect("write");
}

fn rules(v: &serde_json::Value) -> Vec<String> {
    v["violations"]
        .as_array()
        .expect("violations")
        .iter()
        .map(|x| x["label"].as_str().expect("label").to_owned())
        .collect()
}

/// Cites: CON-14
#[test]
fn a_spec_edit_without_the_spec_change_label_fails() {
    let dir = root_with(FULL_CODEOWNERS, false);
    let changed = "specs/010-trace-schema.md,crates/acn-trace/src/lib.rs";
    let bad = xtask_at(
        dir.path(),
        &["pr-check", "--changed", changed, "--labels", "lab"],
    );
    assert!(!bad.ok(), "{}", bad.json);
    assert_eq!(rules(&bad.json), vec!["spec-change"]);
    assert_eq!(
        strings(&bad.json["violations"][0], "paths"),
        vec!["specs/010-trace-schema.md"]
    );

    let good = xtask_at(
        dir.path(),
        &["pr-check", "--changed", changed, "--labels", "spec-change"],
    );
    assert!(good.ok(), "{}", good.json);
}

/// Cites: CON-7
#[test]
fn a_frozen_set_edit_needs_env_change_once_m0_is_closed() {
    let changed = "hypotheses/p4.toml,env-hash.json,crates/acn-attrib/src/core/mod.rs,crates/acn-attrib/src/lib.rs";

    let before = root_with(FULL_CODEOWNERS, false);
    let advisory = xtask_at(
        before.path(),
        &["pr-check", "--changed", changed, "--labels", ""],
    );
    assert!(
        advisory.ok(),
        "pre-M0 the rule is advisory: {}",
        advisory.json
    );
    assert_eq!(
        advisory.json["advisories"]
            .as_array()
            .expect("advisories")
            .len(),
        1
    );

    let after = root_with(FULL_CODEOWNERS, true);
    let bad = xtask_at(
        after.path(),
        &["pr-check", "--changed", changed, "--labels", "spec-change"],
    );
    assert!(!bad.ok(), "{}", bad.json);
    assert_eq!(rules(&bad.json), vec!["env-change"]);
    let paths = strings(&bad.json["violations"][0], "paths");
    assert_eq!(
        paths.len(),
        3,
        "lib.rs outside core/ is not frozen: {paths:?}"
    );

    let good = xtask_at(
        after.path(),
        &["pr-check", "--changed", changed, "--labels", "env-change"],
    );
    assert!(good.ok(), "{}", good.json);
}

/// Cites: CON-7, CON-14
#[test]
fn ordinary_changes_need_no_label() {
    let dir = root_with(FULL_CODEOWNERS, true);
    let run = xtask_at(
        dir.path(),
        &[
            "pr-check",
            "--changed",
            "crates/acn-emu/src/lib.rs,docs/lab/x.md",
            "--labels",
            "",
        ],
    );
    assert!(run.ok(), "{}", run.json);
}

/// Cites: LOOP-20
#[test]
fn codeowners_must_cover_every_protected_path() {
    let partial = FULL_CODEOWNERS.replace("/specs/                          @owner\n", "");
    let dir = root_with(&partial, false);
    let run = xtask_at(dir.path(), &["pr-check"]);
    assert!(!run.ok(), "{}", run.json);
    assert_eq!(strings(&run.json, "codeowners_missing"), vec!["/specs/"]);

    // A pattern without an owner does not count.
    let ownerless = FULL_CODEOWNERS.replace("/specs/                          @owner", "/specs/");
    let dir = root_with(&ownerless, false);
    let run = xtask_at(dir.path(), &["pr-check"]);
    assert_eq!(strings(&run.json, "codeowners_missing"), vec!["/specs/"]);

    let none = tempfile::tempdir().expect("tempdir");
    let run = xtask_at(none.path(), &["pr-check"]);
    assert!(!run.ok(), "a missing CODEOWNERS file fails: {}", run.json);
}

/// Cites: LOOP-20
#[test]
fn self_host_codeowners_covers_the_protected_paths() {
    let run = xtask_at(&repo_root(), &["pr-check"]);
    assert!(run.ok(), "{}", run.json);
}

/// Cites: CON-8
#[test]
fn pr_check_base_mode_reads_the_diff_from_git() {
    // Against HEAD itself the diff is empty: nothing to label, exit 0, one JSON object.
    let run = xtask_at(
        &repo_root(),
        &["pr-check", "--base", "HEAD", "--labels", ""],
    );
    assert!(run.ok(), "{}", run.json);
    assert_eq!(run.json["changed"], 0, "{}", run.json);
}
