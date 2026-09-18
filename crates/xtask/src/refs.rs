//! Requirement-ID references outside Rust sources (ADR-3, amendment 2).
//!
//! Markdown at the root, under `docs/`, `specs/` and `.github/`, and hypothesis files
//! under `hypotheses/`, are scanned for `PREFIX-n` tokens. A token whose prefix
//! belongs to a written spec must name a defined ID; a token whose prefix is
//! only listed in `specs/README.md` (a spec still to write) is a forward
//! reference; any other token (`UTF-8`, `ADR-3`, `H-1`) is not a requirement
//! ID. `docs/generated/`, `docs/lab/` and `lab/` are not scanned (CON-23).

use std::collections::BTreeSet;
use std::path::Path;

use serde::Serialize;
use walkdir::WalkDir;

use crate::Result;
use crate::specs::Requirement;
use crate::workspace::{read, rel};

/// One reference to a requirement ID in a non-Rust file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reference {
    pub id: String,
    pub file: String,
    pub line: usize,
}

/// A hypothesis file whose `spec = "..."` names a spec that is neither present nor indexed.
#[derive(Debug, Clone, Serialize)]
pub struct DanglingSpecFile {
    pub spec: String,
    pub file: String,
    pub line: usize,
}

/// Result of the reference scan.
#[derive(Debug, Default, Serialize)]
pub struct RefScan {
    pub dangling: Vec<Reference>,
    pub dangling_spec_files: Vec<DanglingSpecFile>,
    /// Unique forward references: IDs of unwritten specs and unwritten spec files.
    pub forward: BTreeSet<String>,
    pub files_scanned: usize,
}

/// Every `PREFIX-n` token in `line` (word-bounded; trailing `(a)`, `c`, `..n` are not part of it).
pub fn id_tokens(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let boundary = i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'-');
        if !(boundary && bytes[i].is_ascii_uppercase()) {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_uppercase() || bytes[i].is_ascii_digit()) {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'-' {
            let digits_start = i + 1;
            let mut j = digits_start;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > digits_start {
                out.push(line[start..j].to_owned());
                // A range, `TRC-10..21` or `TRC-10–21`, names its second endpoint too.
                let rest = &line[j..];
                let after_sep = rest.strip_prefix("..").or_else(|| rest.strip_prefix('–'));
                if let Some(tail) = after_sep {
                    let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
                    if !digits.is_empty() {
                        out.push(format!("{}{digits}", &line[start..digits_start]));
                        j += rest.len() - tail.len() + digits.len();
                    }
                }
                i = j;
            }
        }
    }
    out
}

/// Prefixes and spec file names listed in the `specs/README.md` index table.
fn index(root: &Path) -> Result<(BTreeSet<String>, BTreeSet<String>)> {
    let mut prefixes = BTreeSet::new();
    let mut files = BTreeSet::new();
    let path = root.join("specs/README.md");
    if !path.is_file() {
        return Ok((prefixes, files));
    }
    for line in read(&path)?.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() < 5 || !cells[1].chars().next().is_some_and(|c| c.is_ascii_digit()) {
            continue;
        }
        if cells[2].ends_with(".md") {
            files.insert(format!("specs/{}", cells[2]));
        }
        // A prefix cell holds whole prefixes separated by commas or spaces; a
        // placeholder such as `P…` is not a prefix.
        for p in cells[3].split(|c: char| c == ',' || c.is_whitespace()) {
            let whole = !p.is_empty()
                && p.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && p.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
            if whole {
                prefixes.insert(p.to_owned());
            }
        }
    }
    Ok((prefixes, files))
}

fn scanned_files(root: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        let mut top: Vec<_> = entries
            .filter_map(std::result::Result::ok)
            .map(|e| e.path())
            .collect();
        top.sort();
        files.extend(
            top.into_iter()
                .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "md")),
        );
    }
    for (base, ext) in [
        ("docs", "md"),
        ("specs", "md"),
        (".github", "md"),
        ("hypotheses", "toml"),
    ] {
        let dir = root.join(base);
        if !dir.is_dir() {
            continue;
        }
        let walker = WalkDir::new(&dir)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_str().unwrap_or("");
                !(e.file_type().is_dir()
                    && e.depth() > 0
                    && (name == "generated" || name == "lab" || name.starts_with('.')))
            });
        for entry in walker {
            let entry = entry?;
            if entry.file_type().is_file() && entry.path().extension().is_some_and(|x| x == ext) {
                files.push(entry.path().to_path_buf());
            }
        }
    }
    Ok(files)
}

/// Scan `root` for ID references and classify them against `reqs`.
pub fn scan(root: &Path, reqs: &[Requirement]) -> Result<RefScan> {
    let known_ids: BTreeSet<&str> = reqs.iter().map(|r| r.id.as_str()).collect();
    let written_prefixes: BTreeSet<&str> = reqs.iter().map(|r| r.prefix.as_str()).collect();
    let (indexed_prefixes, indexed_files) = index(root)?;
    let mut out = RefScan::default();
    for path in scanned_files(root)? {
        let file = rel(root, &path);
        let text = read(&path)?;
        out.files_scanned += 1;
        let is_hypothesis = file.starts_with("hypotheses/");
        // Inside specs/, fenced code is example text (the spec parser ignores it
        // too). Elsewhere fences hold real references, e.g. the task list in TASKS.md.
        let skip_fences = file.starts_with("specs/");
        let mut in_fence = false;
        for (n, line) in text.lines().enumerate() {
            if skip_fences && line.trim_start().starts_with("```") {
                in_fence = !in_fence;
                continue;
            }
            if in_fence {
                continue;
            }
            for id in id_tokens(line) {
                let prefix = id.split_once('-').map_or("", |(p, _)| p);
                if written_prefixes.contains(prefix) {
                    if !known_ids.contains(id.as_str()) {
                        out.dangling.push(Reference {
                            id,
                            file: file.clone(),
                            line: n + 1,
                        });
                    }
                } else if indexed_prefixes.contains(prefix) {
                    out.forward.insert(id);
                }
            }
        }
        if is_hypothesis {
            match poc_spec(&text) {
                Ok(None) => {}
                Ok(Some(spec)) if is_spec_path(&spec) && root.join(&spec).is_file() => {}
                Ok(Some(spec)) if is_spec_path(&spec) && indexed_files.contains(&spec) => {
                    out.forward.insert(spec);
                }
                Ok(Some(spec)) => out.dangling_spec_files.push(DanglingSpecFile {
                    spec,
                    file: file.clone(),
                    line: 0,
                }),
                Err(e) => out.dangling_spec_files.push(DanglingSpecFile {
                    spec: format!("<unparseable TOML: {e}>"),
                    file: file.clone(),
                    line: 0,
                }),
            }
        }
    }
    Ok(out)
}

/// `[poc].spec` of a hypothesis file, read as TOML (any quoting, inline tables).
fn poc_spec(text: &str) -> std::result::Result<Option<String>, toml::de::Error> {
    let value: toml::Value = toml::from_str(text.trim_start_matches('\u{feff}'))?;
    Ok(value
        .get("poc")
        .and_then(|poc| poc.get("spec"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned))
}

/// A spec path is `specs/<file>.md`: one component below `specs/`, no traversal.
fn is_spec_path(spec: &str) -> bool {
    spec.strip_prefix("specs/").is_some_and(|f| {
        !f.is_empty() && !f.contains('/') && !f.contains("..") && f.ends_with(".md")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens() {
        assert_eq!(
            id_tokens("see CON-5(e), TRC-10..21 and CON-5c; (LOOP-3), HYP-1–9"),
            [
                "CON-5", "TRC-10", "TRC-21", "CON-5", "LOOP-3", "HYP-1", "HYP-9"
            ]
        );
        assert_eq!(
            id_tokens("x86-64 aCON-1 pre-CON-2 UTF-8 P1A-3"),
            ["UTF-8", "P1A-3"]
        );
        assert!(id_tokens("L-M0 H- -5 CON-").is_empty());
    }

    #[test]
    fn poc_spec_is_read_as_toml() {
        assert_eq!(
            poc_spec("[poc]\nspec = 'specs/a.md'\n")
                .ok()
                .flatten()
                .as_deref(),
            Some("specs/a.md")
        );
        assert_eq!(
            poc_spec("poc = { spec = \"specs/b.md\" }\n")
                .ok()
                .flatten()
                .as_deref(),
            Some("specs/b.md")
        );
        assert_eq!(
            poc_spec("[other]\nspec = \"specs/c.md\"\n").ok().flatten(),
            None
        );
        assert!(poc_spec("[poc\n").is_err());
    }

    #[test]
    fn spec_paths() {
        assert!(is_spec_path("specs/100-p4.md"));
        assert!(!is_spec_path("specs/../hypotheses/p1.toml"));
        assert!(!is_spec_path("specs/sub/x.md"));
        assert!(!is_spec_path("elsewhere/x.md"));
    }
}
