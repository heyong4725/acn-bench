//! `cargo xtask trace-check` (CON-12): every in-scope requirement has a citing
//! test, every citation names a real ID and sits on a test function, and every
//! ID reference in the docs and hypothesis files resolves (ADR-3).

use std::path::Path;

use serde::Serialize;

use crate::Result;
use crate::citations::{Citation, Problem};
use crate::model::Model;
use crate::refs::{self, DanglingSpecFile, Reference};
use crate::specs::Level;

/// An in-scope requirement with no citing test.
#[derive(Debug, Serialize)]
pub struct Missing {
    pub id: String,
    pub file: String,
    pub line: usize,
    pub level: Level,
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
    pub dangling_references: Vec<Reference>,
    pub dangling_spec_files: Vec<DanglingSpecFile>,
    pub forward_references: Vec<String>,
}

/// Run the check against `root`.
pub fn run(root: &Path) -> Result<Report> {
    let model = Model::load(root)?;
    let refs = refs::scan(root, &model.requirements)?;
    let by_id = model.citations_by_id();
    // The scope file is the declaration of what is implemented, so every ID in
    // it needs a test whatever its wording: profile-style requirements
    // ("Required: …") carry no RFC 2119 keyword and would otherwise never be enforced.
    let missing: Vec<Missing> = model
        .requirements
        .iter()
        .filter(|r| model.implemented.contains(&r.id) && !by_id.contains_key(r.id.as_str()))
        .map(|r| Missing {
            id: r.id.clone(),
            file: format!("specs/{}", r.file),
            line: r.line,
            level: r.level,
        })
        .collect();
    let unknown_citations: Vec<Citation> = model.unknown_citations().into_iter().cloned().collect();
    let ok = missing.is_empty()
        && unknown_citations.is_empty()
        && model.problems.is_empty()
        && model.scope_errors.is_empty()
        && refs.dangling.is_empty()
        && refs.dangling_spec_files.is_empty();
    for m in &missing {
        tracing::error!(id = %m.id, at = %format!("{}:{}", m.file, m.line), "in-scope requirement has no citing test");
    }
    for c in &unknown_citations {
        tracing::error!(id = %c.id, at = %format!("{}:{}", c.file, c.line), "citation names an ID no spec defines");
    }
    for r in &refs.dangling {
        tracing::error!(id = %r.id, at = %format!("{}:{}", r.file, r.line), "reference to an ID its spec does not define");
    }
    for d in &refs.dangling_spec_files {
        tracing::error!(spec = %d.spec, at = %format!("{}:{}", d.file, d.line), "hypothesis names a spec that is neither present nor listed in specs/README.md");
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
        files_scanned: model.files_scanned + refs.files_scanned,
        missing,
        unknown_citations,
        problems: model.problems,
        scope_errors: model.scope_errors,
        dangling_references: refs.dangling,
        dangling_spec_files: refs.dangling_spec_files,
        forward_references: refs.forward.into_iter().collect(),
    })
}
