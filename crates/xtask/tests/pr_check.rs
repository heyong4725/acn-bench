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
/docs/gates/                     @owner
/.github/CODEOWNERS              @owner
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

// ---- git-mode tests on a real temporary repository (review round 1: B1, B2, S3) ----

fn git(root: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.com",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .expect("git");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A repo whose `main` has a spec, a frozen hypothesis, CODEOWNERS and (optionally) a closed M0 gate,
/// with a `work` branch checked out on top.
fn git_repo(m0_closed: bool) -> tempfile::TempDir {
    let dir = root_with(FULL_CODEOWNERS, m0_closed);
    write(dir.path(), "specs/000-constitution.md", "spec\n");
    write(dir.path(), "hypotheses/p4.toml", "[poc]\nid = \"p4\"\n");
    write(dir.path(), "README.md", "readme\n");
    git(dir.path(), &["init", "-q", "-b", "main"]);
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-q", "-m", "base"]);
    git(dir.path(), &["switch", "-q", "-c", "work"]);
    dir
}

/// Cites: CON-14, CON-7
#[test]
fn moving_a_file_out_of_a_protected_directory_is_still_a_change_to_it() {
    let dir = git_repo(true);
    git(
        dir.path(),
        &["mv", "specs/000-constitution.md", "moved-spec.md"],
    );
    git(dir.path(), &["mv", "hypotheses/p4.toml", "moved-hyp.toml"]);
    git(dir.path(), &["commit", "-q", "-m", "move"]);
    let run = xtask_at(dir.path(), &["pr-check", "--base", "main", "--labels", ""]);
    assert!(!run.ok(), "{}", run.json);
    let mut labels = rules(&run.json);
    labels.sort();
    assert_eq!(labels, vec!["env-change", "spec-change"], "{}", run.json);
}

/// Cites: CON-14
#[test]
fn non_ascii_paths_are_not_hidden_by_git_quoting() {
    let dir = git_repo(false);
    write(dir.path(), "specs/020-café.md", "new spec\n");
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-q", "-m", "add"]);
    let run = xtask_at(dir.path(), &["pr-check", "--base", "main", "--labels", ""]);
    assert!(!run.ok(), "{}", run.json);
    assert_eq!(rules(&run.json), vec!["spec-change"]);
}

/// Cites: CON-7
#[test]
fn deleting_the_m0_gate_record_does_not_reopen_the_frozen_set() {
    let dir = git_repo(true);
    git(dir.path(), &["rm", "-q", "docs/gates/M0.md"]);
    write(
        dir.path(),
        "hypotheses/p4.toml",
        "[poc]\nid = \"p4\"\n# edited\n",
    );
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-q", "-m", "sneaky"]);
    let run = xtask_at(dir.path(), &["pr-check", "--base", "main", "--labels", ""]);
    assert!(
        !run.ok(),
        "M0 state must come from the base, not the PR head: {}",
        run.json
    );
    assert_eq!(rules(&run.json), vec!["env-change"]);
    assert_eq!(run.json["m0_closed"], true);
}

/// Cites: CON-8
#[test]
fn a_base_that_looks_like_an_option_is_rejected() {
    let dir = git_repo(false);
    let run = xtask_at(
        dir.path(),
        &["pr-check", "--base=--output=injected", "--labels", ""],
    );
    assert!(!run.ok(), "{}", run.json);
    assert!(
        run.json["error"]
            .as_str()
            .expect("error")
            .contains("--base"),
        "{}",
        run.json
    );
    assert!(!dir.path().join("injected...HEAD").exists());
}

/// Cites: CON-14
#[test]
fn changed_paths_are_normalised_before_matching() {
    let dir = root_with(FULL_CODEOWNERS, false);
    for p in ["./specs/010.md", "docs/../specs/010.md", "Specs/010.md"] {
        let run = xtask_at(dir.path(), &["pr-check", "--changed", p, "--labels", ""]);
        assert!(
            !run.ok(),
            "`{p}` must count as a specs/ change: {}",
            run.json
        );
    }
    let run = xtask_at(
        dir.path(),
        &[
            "pr-check",
            "--changed",
            "specs-old/x.md,hypotheses2/y.toml",
            "--labels",
            "",
        ],
    );
    assert!(run.ok(), "{}", run.json);
}

/// Cites: LOOP-20
#[test]
fn codeowners_coverage_follows_last_match_wins_and_needs_a_real_owner() {
    for (what, extra) in [
        ("a later ownerless entry", "/specs/\n"),
        ("a later catch-all", "* @bot\n"),
        ("a later narrower pattern", "/specs/*.md @bot\n"),
        ("a later parent pattern", "/crates/ @bot\n"),
    ] {
        let dir = root_with(&format!("{FULL_CODEOWNERS}{extra}"), false);
        let run = xtask_at(dir.path(), &["pr-check"]);
        assert!(!run.ok(), "{what} must break coverage: {}", run.json);
    }
    let commented =
        FULL_CODEOWNERS.replace("/specs/                          @owner", "/specs/ # TODO");
    let dir = root_with(&commented, false);
    let run = xtask_at(dir.path(), &["pr-check"]);
    assert_eq!(
        strings(&run.json, "codeowners_missing"),
        vec!["/specs/"],
        "a comment is not an owner"
    );

    // An earlier catch-all is fine: the exact entry comes last.
    let dir = root_with(&format!("* @everyone\n{FULL_CODEOWNERS}"), false);
    assert!(xtask_at(dir.path(), &["pr-check"]).ok());
}

/// Cites: LOOP-20
#[test]
fn the_gate_records_and_codeowners_itself_are_protected() {
    let partial = FULL_CODEOWNERS.replace("/docs/gates/                     @owner\n", "");
    let dir = root_with(&partial, false);
    let run = xtask_at(dir.path(), &["pr-check"]);
    assert_eq!(
        strings(&run.json, "codeowners_missing"),
        vec!["/docs/gates/"]
    );
}
