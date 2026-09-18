//! `cargo xtask pr-check` (ADR-6): the label rules for a pull request and the
//! CODEOWNERS coverage of protected paths.
//!
//! - CON-14: a change under `specs/` needs the `spec-change` label.
//! - CON-7: a change to the frozen set or to `env-hash.json` needs the
//!   `env-change` label once the M0 gate is closed (`docs/gates/M0.md` exists);
//!   before that it is reported as an advisory.
//! - LOOP-20: `.github/CODEOWNERS` must assign an owner to every protected path.

use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::env_hash::{FROZEN_SET, RECORD_FILE};
use crate::scope::SCOPE_FILE;
use crate::workspace::read;
use crate::{Error, Result};

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
    /// `git diff --name-only <base>...HEAD`.
    GitBase(String),
}

/// Paths CODEOWNERS must cover: the frozen set, the specs, and the two records.
pub fn protected_patterns() -> Vec<String> {
    let mut v: Vec<String> = FROZEN_SET.iter().map(|p| format!("/{p}/")).collect();
    v.push("/specs/".to_owned());
    v.push(format!("/{RECORD_FILE}"));
    v.push(format!("/{SCOPE_FILE}"));
    v
}

fn normalise(pattern: &str) -> String {
    pattern
        .trim_start_matches('/')
        .trim_end_matches('/')
        .to_owned()
}

/// Protected patterns that have no owned entry in `.github/CODEOWNERS`.
pub fn codeowners_missing(root: &Path) -> Result<Vec<String>> {
    let path = root.join(".github/CODEOWNERS");
    let text = if path.is_file() {
        read(&path)?
    } else {
        String::new()
    };
    let owned: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            let pattern = parts.next()?;
            parts.next().map(|_owner| normalise(pattern))
        })
        .collect();
    Ok(protected_patterns()
        .into_iter()
        .filter(|p| !owned.contains(&normalise(p)))
        .collect())
}

fn git_changed(root: &Path, base: &str) -> Result<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["diff", "--name-only", &format!("{base}...HEAD")])
        .output()
        .map_err(|e| Error::io(root, e))?;
    if !out.status.success() {
        return Err(Error::Invalid(format!(
            "`git diff --name-only {base}...HEAD` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .filter(|l| !l.is_empty())
        .collect())
}

fn is_frozen(path: &str) -> bool {
    path == RECORD_FILE
        || FROZEN_SET.iter().any(|f| {
            path.strip_prefix(f)
                .is_some_and(|rest| rest.starts_with('/'))
        })
}

/// Run the check.
pub fn run(root: &Path, changes: Changes, labels: &[String]) -> Result<Report> {
    let changed = match changes {
        Changes::None => Vec::new(),
        Changes::List(v) => v,
        Changes::GitBase(base) => git_changed(root, &base)?,
    };
    let m0_closed = root.join("docs/gates/M0.md").is_file();
    let has = |l: &str| labels.iter().any(|x| x == l);
    let mut violations = Vec::new();
    let mut advisories = Vec::new();

    let spec_paths: Vec<String> = changed
        .iter()
        .filter(|p| p.starts_with("specs/"))
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
    for m in &missing {
        tracing::error!(pattern = %m, "CODEOWNERS has no owned entry for a protected path (LOOP-20)");
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
}
