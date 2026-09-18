//! `cargo xtask pr-check` (ADR-6): the label rules for a pull request and the
//! CODEOWNERS coverage of protected paths.
//!
//! - CON-14: a change under `specs/` needs the `spec-change` label.
//! - CON-7: a change to the frozen set or to `env-hash.json` needs the
//!   `env-change` label once the M0 gate is closed (`docs/gates/M0.md` exists
//!   on the base or on the head); before that it is reported as an advisory.
//! - LOOP-20: `.github/CODEOWNERS` must assign an owner to every protected
//!   path, under GitHub's last-match-wins rule.

use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::env_hash::{FROZEN_SET, RECORD_FILE};
use crate::scope::SCOPE_FILE;
use crate::workspace::read;
use crate::{Error, Result};

const M0_RECORD: &str = "docs/gates/M0.md";
const CODEOWNERS: &str = ".github/CODEOWNERS";

/// A label rule that the PR violates (or, as an advisory, would violate after M0).
#[derive(Debug, Serialize)]
pub struct Finding {
    pub rule: &'static str,
    pub label: &'static str,
    pub paths: Vec<String>,
    pub message: String,
}

/// The JSON object `pr-check` prints (CON-8).
#[derive(Debug, Serialize)]
pub struct Report {
    pub ok: bool,
    pub changed: usize,
    pub labels: Vec<String>,
    pub m0_closed: bool,
    pub violations: Vec<Finding>,
    pub advisories: Vec<Finding>,
    pub codeowners_missing: Vec<String>,
}

/// Where the changed-path list comes from.
#[derive(Debug, Clone)]
pub enum Changes {
    /// Only the static CODEOWNERS check.
    None,
    /// An explicit list of repo-relative paths.
    List(Vec<String>),
    /// `git diff --no-renames --name-only <base>...HEAD`.
    GitBase(String),
}

/// Paths CODEOWNERS must cover: the frozen set, the specs, the gate records,
/// the two records the gates read, and CODEOWNERS itself.
pub fn protected_patterns() -> Vec<String> {
    let mut v: Vec<String> = FROZEN_SET.iter().map(|p| format!("/{p}/")).collect();
    v.push("/specs/".to_owned());
    v.push(format!("/{RECORD_FILE}"));
    v.push(format!("/{SCOPE_FILE}"));
    v.push("/docs/gates/".to_owned());
    v.push(format!("/{CODEOWNERS}"));
    v
}

fn normalise_pattern(pattern: &str) -> String {
    pattern
        .trim_start_matches('/')
        .trim_end_matches('/')
        .to_owned()
}

/// Whether a CODEOWNERS pattern could decide ownership of files under `protected`
/// (both normalised). Conservative: globs, parents and children all count.
fn overlaps(pattern: &str, protected: &str) -> bool {
    let literal: String = pattern
        .chars()
        .take_while(|c| *c != '*' && *c != '?' && *c != '[')
        .collect();
    let literal = literal.trim_end_matches('/');
    literal.is_empty()
        || literal == protected
        || protected.starts_with(&format!("{literal}/"))
        || literal.starts_with(&format!("{protected}/"))
}

/// Protected patterns whose *last* overlapping CODEOWNERS entry is not an exact,
/// owned entry (GitHub applies the last matching pattern).
pub fn codeowners_missing(root: &Path) -> Result<Vec<String>> {
    let path = root.join(CODEOWNERS);
    let text = if path.is_file() {
        read(&path)?
    } else {
        String::new()
    };
    // (pattern, has a real owner)
    let entries: Vec<(String, bool)> = text
        .lines()
        .map(|l| l.split_once('#').map_or(l, |(code, _)| code).trim())
        .filter(|l| !l.is_empty())
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            let pattern = normalise_pattern(parts.next()?);
            let owned = parts.any(|o| o.starts_with('@') || (o.contains('@') && o.contains('.')));
            Some((pattern, owned))
        })
        .collect();
    Ok(protected_patterns()
        .into_iter()
        .filter(|p| {
            let protected = normalise_pattern(p);
            let last = entries
                .iter()
                .rev()
                .find(|(pat, _)| overlaps(pat, &protected));
            !matches!(last, Some((pat, true)) if *pat == protected)
        })
        .collect())
}

fn git(root: &Path, args: &[&str]) -> Result<std::process::Output> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| Error::io(root, e))
}

fn first_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_owned()
}

fn check_base(base: &str) -> Result<()> {
    if base.is_empty() || base.starts_with('-') {
        return Err(Error::Invalid(format!(
            "--base must be a git ref, got `{base}`"
        )));
    }
    Ok(())
}

/// Paths touched between the merge base and HEAD. `--no-renames` reports both
/// sides of a move; `-z` disables git's quoting of non-ASCII paths.
fn git_changed(root: &Path, base: &str) -> Result<Vec<String>> {
    check_base(base)?;
    let range = format!("{base}...HEAD");
    let out = git(
        root,
        &[
            "diff",
            "--no-renames",
            "--name-only",
            "-z",
            "--end-of-options",
            &range,
        ],
    )?;
    if !out.status.success() {
        return Err(Error::Invalid(format!(
            "cannot diff against --base `{base}` (is this a git checkout with that ref fetched?): {}",
            first_line(&out.stderr)
        )));
    }
    Ok(out
        .stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect())
}

/// Whether the M0 record exists at the merge base of `base` and HEAD.
fn m0_closed_at_base(root: &Path, base: &str) -> Result<bool> {
    check_base(base)?;
    let mb = git(root, &["merge-base", "--end-of-options", base, "HEAD"])?;
    if !mb.status.success() {
        return Err(Error::Invalid(format!(
            "no merge base between `{base}` and HEAD: {}",
            first_line(&mb.stderr)
        )));
    }
    let object = format!("{}:{M0_RECORD}", first_line(&mb.stdout));
    Ok(git(root, &["cat-file", "-e", &object])?.status.success())
}

/// Lexically normalise a repo-relative path: drop `./`, resolve `..`, forward slashes.
fn normalise_path(path: &str) -> String {
    let unified = path.replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();
    for c in unified.split('/') {
        match c {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// Case-insensitive on purpose: `Specs/x.md` is the same file on the primary platform (CON-1).
fn is_under(path: &str, dir: &str) -> bool {
    let (p, d) = (path.to_ascii_lowercase(), dir.to_ascii_lowercase());
    p.strip_prefix(&d).is_some_and(|rest| rest.starts_with('/'))
}

fn is_frozen(path: &str) -> bool {
    path.eq_ignore_ascii_case(RECORD_FILE) || FROZEN_SET.iter().any(|f| is_under(path, f))
}

/// Run the check.
pub fn run(root: &Path, changes: Changes, labels: &[String]) -> Result<Report> {
    if !root.is_dir() {
        return Err(Error::Invalid(format!(
            "--root {} is not a directory",
            root.display()
        )));
    }
    let mut m0_closed = root.join(M0_RECORD).is_file();
    let changed: Vec<String> = match changes {
        Changes::None => Vec::new(),
        Changes::List(v) => v,
        Changes::GitBase(base) => {
            // The gate state is read from the base as well, so a PR cannot
            // reopen the frozen set by deleting the M0 record.
            m0_closed = m0_closed || m0_closed_at_base(root, &base)?;
            git_changed(root, &base)?
        }
    }
    .iter()
    .map(|p| normalise_path(p))
    .filter(|p| !p.is_empty())
    .collect();

    let has = |l: &str| labels.iter().any(|x| x == l);
    let mut violations = Vec::new();
    let mut advisories = Vec::new();

    let spec_paths: Vec<String> = changed
        .iter()
        .filter(|p| is_under(p, "specs"))
        .cloned()
        .collect();
    if !spec_paths.is_empty() && !has("spec-change") {
        violations.push(Finding {
            rule: "CON-14",
            label: "spec-change",
            paths: spec_paths,
            message: "specs/ changed: label the PR `spec-change` and state the rationale and the IDs added, changed or retired".to_owned(),
        });
    }

    let frozen_paths: Vec<String> = changed.iter().filter(|p| is_frozen(p)).cloned().collect();
    if !frozen_paths.is_empty() && !has("env-change") {
        let finding = Finding {
            rule: "CON-7",
            label: "env-change",
            paths: frozen_paths,
            message: if m0_closed {
                "the frozen set changed: label the PR `env-change`, include the updated `cargo xtask env-hash --write` output, and request an adversarial review".to_owned()
            } else {
                "the frozen set changed; after the M0 gate this will require the `env-change` label (CON-7)".to_owned()
            },
        };
        if m0_closed {
            violations.push(finding)
        } else {
            advisories.push(finding)
        }
    }

    let missing = codeowners_missing(root)?;
    for v in &violations {
        tracing::error!(rule = v.rule, label = v.label, "{}", v.message);
    }
    for a in &advisories {
        tracing::warn!(rule = a.rule, label = a.label, "{}", a.message);
    }
    for m in &missing {
        tracing::error!(pattern = %m, "the last CODEOWNERS entry that can match this protected path is not an exact, owned entry (LOOP-20)");
    }
    Ok(Report {
        ok: violations.is_empty() && missing.is_empty(),
        changed: changed.len(),
        labels: labels.to_vec(),
        m0_closed,
        violations,
        advisories,
        codeowners_missing: missing,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_paths() {
        assert!(is_frozen("hypotheses/p4.toml"));
        assert!(is_frozen("env-hash.json"));
        assert!(is_frozen("crates/acn-attrib/src/core/mod.rs"));
        assert!(!is_frozen("crates/acn-attrib/src/lib.rs"));
        assert!(!is_frozen("hypotheses-old/x.toml"));
        assert!(!is_frozen("lab/hypotheses/p17.toml"));
    }

    #[test]
    fn path_normalisation() {
        assert_eq!(normalise_path("./specs/a.md"), "specs/a.md");
        assert_eq!(normalise_path("docs/../specs/a.md"), "specs/a.md");
        assert_eq!(normalise_path("../../x"), "x");
    }

    #[test]
    fn overlap() {
        assert!(overlaps("*", "specs"));
        assert!(overlaps("specs/*.md", "specs"));
        assert!(overlaps("crates", "crates/acn-hyp"));
        assert!(!overlaps("crates/acn-emu", "crates/acn-hyp"));
        assert!(!overlaps("specs-old", "specs"));
    }
}
