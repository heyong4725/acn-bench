//! `cargo xtask trace-check` (CON-12): every implemented MUST has a citing
//! test, every citation names a real ID, and every citation sits on a test
//! function in code the compiler sees. Every failure is logged as well as reported.

use std::path::Path;

use serde::Serialize;

use crate::Result;
use crate::citations::{Citation, Problem};
use crate::model::Model;
use crate::specs::Level;

/// An implemented MUST with no citing test.
#[derive(Debug, Serialize)]
pub struct Missing {
    pub id: String,
    pub file: String,
    pub line: usize,
}

/// The JSON object `trace-check` prints (CON-8).
#[derive(Debug, Serialize)]
pub struct Report {
    pub ok: bool,
    pub root: String,
    pub specs: usize,
    pub ids: usize,
    pub must_ids: usize,
    pub implemented: usize,
    pub cited_ids: usize,
    pub files_scanned: usize,
    pub missing: Vec<Missing>,
    pub unknown_citations: Vec<Citation>,
    pub problems: Vec<Problem>,
    pub scope_errors: Vec<String>,
}

/// Run the check against `root`.
pub fn run(root: &Path) -> Result<Report> {
    let model = Model::load(root)?;
    let by_id = model.citations_by_id();
    let missing: Vec<Missing> = model
        .requirements
        .iter()
        .filter(|r| {
            r.level == Level::Must
                && model.implemented.contains(&r.id)
                && !by_id.contains_key(r.id.as_str())
        })
        .map(|r| Missing {
            id: r.id.clone(),
            file: format!("specs/{}", r.file),
            line: r.line,
        })
        .collect();
    let unknown_citations: Vec<Citation> = model.unknown_citations().into_iter().cloned().collect();
    let ok = missing.is_empty()
        && unknown_citations.is_empty()
        && model.problems.is_empty()
        && model.scope_errors.is_empty();
    for m in &missing {
        tracing::error!(id = %m.id, at = %format!("{}:{}", m.file, m.line), "implemented MUST has no citing test");
    }
    for c in &unknown_citations {
        tracing::error!(id = %c.id, at = %format!("{}:{}", c.file, c.line), "citation names an ID no spec defines");
    }
    for p in &model.problems {
        tracing::error!(at = %format!("{}:{}", p.file, p.line), "{}", p.message);
    }
    for e in &model.scope_errors {
        tracing::error!("{e}");
    }
    Ok(Report {
        ok,
        root: root.display().to_string(),
        specs: model.spec_count,
        ids: model.requirements.len(),
        must_ids: model
            .requirements
            .iter()
            .filter(|r| r.level == Level::Must)
            .count(),
        implemented: model.implemented.len(),
        cited_ids: by_id.len(),
        files_scanned: model.files_scanned,
        missing,
        unknown_citations,
        problems: model.problems,
        scope_errors: model.scope_errors,
    })
}
