//! `/// Cites: ID, ID` extraction from Rust sources (CON-12).
//!
//! A citation block is one or more `/// Cites:` doc-comment lines; it must be
//! attached (through further doc lines and attributes) to a `fn` item. Sources
//! under `crates/` and `tests/` are scanned; `lab/` is exempt (CON-23) and
//! directories named `target` or `fixtures` are skipped.

use std::path::Path;

use serde::Serialize;
use walkdir::WalkDir;

use crate::Result;
use crate::specs::is_id;
use crate::workspace::{is_skipped_dir, read, rel};

/// One citation of one ID from one function.
#[derive(Debug, Clone, Serialize)]
pub struct Citation {
    pub id: String,
    pub file: String,
    pub line: usize,
    pub function: Option<String>,
}

/// A malformed or detached citation.
#[derive(Debug, Clone, Serialize)]
pub struct Problem {
    pub file: String,
    pub line: usize,
    pub message: String,
}

/// Everything found in a scan.
#[derive(Debug, Default, Serialize)]
pub struct Scan {
    pub citations: Vec<Citation>,
    pub problems: Vec<Problem>,
    pub files_scanned: usize,
}

fn fn_name(line: &str) -> Option<String> {
    let mut tokens = line.split_whitespace();
    while let Some(tok) = tokens.next() {
        if tok == "fn" {
            let name: String = tokens
                .next()?
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            return (!name.is_empty()).then_some(name);
        }
    }
    None
}

/// Scan one file's text. Pure, for unit tests.
pub fn scan_text(file: &str, text: &str, out: &mut Scan) {
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let Some(body) = lines[i].trim_start().strip_prefix("///") else {
            i += 1;
            continue;
        };
        let Some(list) = body.trim().strip_prefix("Cites:") else {
            i += 1;
            continue;
        };
        let line_no = i + 1;
        let ids: Vec<&str> = list
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .collect();

        // Walk forward to the item the doc comment is attached to.
        let mut j = i + 1;
        let mut function = None;
        while j < lines.len() {
            let u = lines[j].trim_start();
            if u.starts_with("///") || u.is_empty() {
                j += 1;
            } else if u.starts_with("#[") || u.starts_with("#![") {
                while j < lines.len() && !lines[j].trim_end().ends_with(']') {
                    j += 1;
                }
                j += 1;
            } else {
                function = fn_name(u);
                break;
            }
        }

        if ids.is_empty() {
            out.problems.push(Problem {
                file: file.to_owned(),
                line: line_no,
                message: "empty `Cites:` list".to_owned(),
            });
        }
        if function.is_none() {
            out.problems.push(Problem {
                file: file.to_owned(),
                line: line_no,
                message: "citation is not attached to a function".to_owned(),
            });
        }
        for id in ids {
            if is_id(id) {
                out.citations.push(Citation {
                    id: id.to_owned(),
                    file: file.to_owned(),
                    line: line_no,
                    function: function.clone(),
                });
            } else {
                out.problems.push(Problem {
                    file: file.to_owned(),
                    line: line_no,
                    message: format!("malformed requirement ID `{id}`"),
                });
            }
        }
        i += 1;
    }
}

/// Scan `<root>/crates` and `<root>/tests` for citations.
pub fn scan(root: &Path) -> Result<Scan> {
    let mut out = Scan::default();
    for base in ["crates", "tests"] {
        let dir = root.join(base);
        if !dir.is_dir() {
            continue;
        }
        let walker = WalkDir::new(&dir)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| {
                !(e.file_type().is_dir() && e.file_name().to_str().is_some_and(is_skipped_dir))
            });
        for entry in walker {
            let entry = entry?;
            if !entry.file_type().is_file() || entry.path().extension().is_none_or(|x| x != "rs") {
                continue;
            }
            let text = read(entry.path())?;
            out.files_scanned += 1;
            scan_text(&rel(root, entry.path()), &text, &mut out);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attached_and_detached() {
        let text = "/// Cites: A-1, A-2\n/// more docs\n#[tokio::test(\n  start_paused = true\n)]\nasync fn go() {}\n\n/// Cites: B-1\nstruct S;\n\n/// Cites:\n#[test]\nfn empty() {}\n";
        let mut scan = Scan::default();
        scan_text("t.rs", text, &mut scan);
        let ids: Vec<(&str, Option<&str>)> = scan
            .citations
            .iter()
            .map(|c| (c.id.as_str(), c.function.as_deref()))
            .collect();
        assert_eq!(
            ids,
            [("A-1", Some("go")), ("A-2", Some("go")), ("B-1", None)]
        );
        assert_eq!(scan.problems.len(), 2, "{:?}", scan.problems);
    }

    #[test]
    fn fn_names() {
        assert_eq!(
            fn_name("pub async fn run_it<T>(x: T)").as_deref(),
            Some("run_it")
        );
        assert_eq!(fn_name("struct X;"), None);
    }
}
