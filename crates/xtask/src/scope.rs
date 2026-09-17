//! The trace scope file: which spec sections and IDs count as implemented
//! (CON-12 "a MUST in an implemented spec section"). See ADR-3.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

use crate::specs::Requirement;
use crate::workspace::read;
use crate::{Error, Result};

/// File name of the scope file at the workspace root.
pub const SCOPE_FILE: &str = "trace-scope.toml";

/// `trace-scope.toml`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeFile {
    #[serde(default)]
    pub implemented: Vec<ScopeEntry>,
}

/// One `[[implemented]]` entry: sections and/or IDs of one spec.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeEntry {
    /// Spec number, e.g. `"000"`.
    pub spec: String,
    #[serde(default)]
    pub sections: Vec<String>,
    #[serde(default)]
    pub ids: Vec<String>,
}

/// Load the scope file; a missing file is an empty scope.
pub fn load(root: &Path) -> Result<ScopeFile> {
    let path = root.join(SCOPE_FILE);
    if !path.is_file() {
        return Ok(ScopeFile::default());
    }
    let text = read(&path)?;
    toml::from_str(&text).map_err(|e| Error::Toml { path, source: e })
}

/// Resolve a scope to the set of implemented IDs, collecting errors for
/// unknown IDs, IDs filed under the wrong spec, and empty sections.
pub fn resolve(scope: &ScopeFile, reqs: &[Requirement]) -> (BTreeSet<String>, Vec<String>) {
    let mut implemented = BTreeSet::new();
    let mut errors = Vec::new();
    for entry in &scope.implemented {
        for id in &entry.ids {
            match reqs.iter().find(|r| &r.id == id) {
                None => errors.push(format!("scope lists unknown ID {id}")),
                Some(r) if r.spec != entry.spec => {
                    errors.push(format!(
                        "scope lists {id} under spec {} but it is defined in spec {}",
                        entry.spec, r.spec
                    ));
                }
                Some(_) => {
                    implemented.insert(id.clone());
                }
            }
        }
        for section in &entry.sections {
            let mut any = false;
            for r in reqs
                .iter()
                .filter(|r| r.spec == entry.spec && r.section.as_ref() == Some(section))
            {
                implemented.insert(r.id.clone());
                any = true;
            }
            if !any {
                errors.push(format!(
                    "scope lists spec {} section {section}, which defines no IDs",
                    entry.spec
                ));
            }
        }
    }
    (implemented, errors)
}
