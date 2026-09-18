//! Self-hosting tests for `cargo xtask trace-check` (CON-12) and its CLI
//! contract (CON-8). Fixtures live under `tests/fixtures/<case>/`, each a
//! miniature repo root with `specs/`, `tests/` and `trace-scope.toml`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // CON-19: tests are exempt

mod common;

use common::{fixture, repo_root, strings, xtask_at};

/// Cites: CON-12
#[test]
fn passes_when_every_implemented_must_is_cited() {
    let run = xtask_at(&fixture("ok"), &["trace-check"]);
    assert!(run.ok(), "{}", run.json);
    assert!(strings(&run.json, "missing").is_empty());
    assert!(strings(&run.json, "unknown_citations").is_empty());
    // Section scope: FIX-1 (MUST) is implemented, FIX-3 is out of scope, FIX-2 is a MAY.
    assert_eq!(run.json["implemented"], 2, "{}", run.json);
    assert_eq!(run.json["must_ids"], 2, "{}", run.json);
}

/// Cites: CON-12
#[test]
fn fails_on_an_uncited_must_in_an_implemented_section() {
    let run = xtask_at(&fixture("uncited"), &["trace-check"]);
    assert!(!run.ok(), "{}", run.json);
    let missing = run.json["missing"]
        .as_array()
        .expect("missing array")
        .iter()
        .map(|m| m["id"].as_str().expect("id").to_owned())
        .collect::<Vec<_>>();
    assert_eq!(missing, vec!["FIX-1".to_owned()], "{}", run.json);
}

/// Cites: CON-12
#[test]
fn fails_on_a_citation_of_a_nonexistent_id() {
    let run = xtask_at(&fixture("unknown_id"), &["trace-check"]);
    assert!(!run.ok(), "{}", run.json);
    let unknown = run.json["unknown_citations"]
        .as_array()
        .expect("unknown_citations array");
    assert_eq!(unknown.len(), 1, "{}", run.json);
    assert_eq!(unknown[0]["id"], "FIX-9");
    assert!(
        unknown[0]["file"]
            .as_str()
            .expect("file")
            .ends_with("tests/fixture.rs"),
        "{}",
        run.json
    );
    // Nothing is missing: FIX-1 is cited; the failure is the unknown ID alone.
    assert!(strings(&run.json, "missing").is_empty(), "{}", run.json);
}

/// Cites: CON-12
#[test]
fn fails_when_a_citation_is_not_attached_to_a_function() {
    let run = xtask_at(&fixture("detached"), &["trace-check"]);
    assert!(!run.ok(), "{}", run.json);
    let problems = run.json["problems"].as_array().expect("problems array");
    assert_eq!(problems.len(), 1, "{}", run.json);
    assert!(
        problems[0]["message"]
            .as_str()
            .expect("message")
            .contains("not attached to a function"),
        "{}",
        run.json
    );
}

/// Cites: CON-12
#[test]
fn ids_inside_code_fences_are_not_requirements() {
    let run = xtask_at(&fixture("ok"), &["trace-check"]);
    // FIX-1, FIX-2, FIX-3 only; FIX-9 sits inside a fence.
    assert_eq!(run.json["ids"], 3, "{}", run.json);
}

/// Cites: CON-12
#[test]
fn self_host_the_real_workspace_passes() {
    let run = xtask_at(&repo_root(), &["trace-check"]);
    assert!(run.ok(), "{}", run.json);
    assert!(run.json["specs"].as_u64().unwrap_or(0) >= 3, "{}", run.json);
}

/// Cites: CON-8
#[test]
fn trace_check_emits_one_json_object_and_exit_code_tracks_ok() {
    // `xtask_at` asserts the contract on both branches; exercise both.
    let good = xtask_at(&fixture("ok"), &["trace-check"]);
    assert_eq!(good.code, Some(0));
    let bad = xtask_at(&fixture("uncited"), &["trace-check"]);
    assert_eq!(bad.code, Some(1));
}

/// Cites: CON-8
#[test]
fn argument_errors_still_honour_the_json_contract() {
    let run = common::xtask(&["no-such-task"]);
    assert!(!run.ok());
    assert!(run.json["error"].is_string(), "{}", run.json);
    assert!(
        run.stderr.contains("no-such-task")
            || run.json["error"]
                .as_str()
                .is_some_and(|e| e.contains("no-such-task"))
    );
}

/// Cites: CON-12
#[test]
fn fails_when_a_citation_is_on_a_non_test_function() {
    let run = xtask_at(&fixture("not_a_test"), &["trace-check"]);
    assert!(!run.ok(), "{}", run.json);
    let problems = run.json["problems"].as_array().expect("problems array");
    assert_eq!(problems.len(), 2, "{}", run.json);
    assert!(
        problems.iter().all(|p| p["message"]
            .as_str()
            .expect("message")
            .contains("not a test function")),
        "{}",
        run.json
    );
    // The citation itself is not counted, so FIX-1 is also reported missing.
    assert_eq!(run.json["missing"][0]["id"], "FIX-1", "{}", run.json);
}

/// Cites: CON-12
#[test]
fn accepts_continuations_and_attribute_placements() {
    let run = xtask_at(&fixture("ok"), &["trace-check"]);
    assert!(run.ok(), "{}", run.json);
    assert!(strings(&run.json, "problems").is_empty(), "{}", run.json);
}

/// Cites: CON-12
#[test]
fn a_root_without_specs_is_an_error_not_a_pass() {
    let dir = tempfile::tempdir().expect("tempdir");
    let run = xtask_at(dir.path(), &["trace-check"]);
    assert!(!run.ok(), "{}", run.json);
    assert!(
        run.json["error"]
            .as_str()
            .expect("error")
            .contains("no specs/ directory"),
        "{}",
        run.json
    );
}

fn ids_of(v: &serde_json::Value, key: &str) -> Vec<String> {
    let mut ids: Vec<String> = v[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key} array"))
        .iter()
        .map(|x| x["id"].as_str().expect("id").to_owned())
        .collect();
    ids.sort();
    ids
}

/// Cites: CON-12
#[test]
fn an_in_scope_id_needs_a_citation_even_without_an_rfc_keyword() {
    let run = xtask_at(&fixture("no_keyword"), &["trace-check"]);
    assert!(!run.ok(), "{}", run.json);
    assert_eq!(ids_of(&run.json, "missing"), vec!["FIX-1"], "{}", run.json);
}

/// Cites: CON-12
#[test]
fn dangling_id_references_in_docs_and_hypotheses_fail() {
    let run = xtask_at(&fixture("refs"), &["trace-check"]);
    assert!(!run.ok(), "{}", run.json);
    // FIX-77 (PLAN.md), FIX-88 (ADR), FIX-66 (hypothesis comment); generated docs, lab notes and lab/ are not scanned.
    assert_eq!(
        ids_of(&run.json, "dangling_references"),
        vec!["FIX-66", "FIX-77", "FIX-88"],
        "{}",
        run.json
    );
    // The only other failure is the hypothesis pointing at a spec that is neither present nor indexed.
    let files = run.json["dangling_spec_files"].as_array().expect("array");
    assert_eq!(files.len(), 1, "{}", run.json);
    assert_eq!(files[0]["spec"], "specs/999-nowhere.md");
    assert_eq!(files[0]["file"], "hypotheses/p2.toml");
}

/// Cites: CON-12
#[test]
fn references_to_indexed_but_unwritten_specs_are_forward_not_dangling() {
    let run = xtask_at(&fixture("refs"), &["trace-check"]);
    let forward = strings(&run.json, "forward_references");
    assert!(forward.contains(&"LTR-4".to_owned()), "{}", run.json);
    assert!(
        forward.contains(&"specs/910-later.md".to_owned()),
        "{}",
        run.json
    );
    for not_an_id in ["UTF-8", "ADR-3", "H-1", "SHA-256"] {
        assert!(!forward.contains(&not_an_id.to_owned()), "{}", run.json);
    }
}
