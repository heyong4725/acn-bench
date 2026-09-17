//! `/// Cites: ID, ID` extraction from Rust sources (CON-12).
//!
//! A citation block is one or more `/// Cites:` doc-comment lines (a line
//! ending in `,` continues on the next doc line). It must be attached, through
//! further doc lines and attributes, to a `fn` that carries a test attribute
//! (`#[test]`, `#[tokio::test(...)]`, …); attributes may sit before or after
//! the doc block. Sources under `crates/` and `tests/` are scanned; `lab/` is
//! exempt (CON-23) and directories named `target` or `fixtures` are skipped.

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

/// Strip a trailing `// comment`, ignoring `//` inside string literals.
fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_str = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if in_str => i += 1,
            b'"' => in_str = !in_str,
            b'/' if !in_str && bytes.get(i + 1) == Some(&b'/') => return line[..i].trim(),
            _ => {}
        }
        i += 1;
    }
    line.trim()
}

/// Byte index of the `]` that closes an attribute opening at the start of
/// `text` (`#[` or `#![`), string-aware and bracket-depth-aware.
fn attr_end(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if in_str => i += 1,
            b'"' => in_str = !in_str,
            b'[' if !in_str => depth += 1,
            b']' if !in_str => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn fn_name(line: &str) -> Option<String> {
    let mut tokens = strip_comment(line).split_whitespace();
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

fn is_doc(line: &str) -> bool {
    line.trim_start().starts_with("///")
}

fn is_attr_start(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("#[") || t.starts_with("#![")
}

/// Consume an attribute starting at `lines[j]`. Returns the attribute text,
/// any code left on its last line after the closing `]`, and the next index.
fn take_attr(lines: &[&str], mut j: usize) -> (String, String, usize) {
    let mut text = String::new();
    while j < lines.len() {
        text.push_str(strip_comment(lines[j]));
        j += 1;
        if let Some(end) = attr_end(&text) {
            let rest = text[end + 1..].trim().to_owned();
            text.truncate(end + 1);
            return (text, rest, j);
        }
        text.push(' ');
    }
    (text, String::new(), j)
}

/// The attribute paths an attribute applies: `#[tokio::test(x)]` → `tokio::test`;
/// `#[cfg_attr(pred, a, b(x))]` → `a`, `b`. `#[cfg(test)]` → `cfg`.
fn attr_paths(attr: &str) -> Vec<String> {
    let inner = attr
        .trim()
        .trim_start_matches("#![")
        .trim_start_matches("#[")
        .trim_end_matches(']')
        .trim();
    let path_of = |s: &str| -> String {
        s.trim()
            .split(|c: char| c == '(' || c == '=' || c.is_whitespace())
            .next()
            .unwrap_or("")
            .to_owned()
    };
    if let Some(args) = inner.strip_prefix("cfg_attr") {
        let args = args.trim().trim_start_matches('(').trim_end_matches(')');
        return args
            .split(',')
            .skip(1)
            .map(path_of)
            .filter(|p| !p.is_empty())
            .collect();
    }
    vec![path_of(inner)]
}

/// A test attribute is one whose path ends in `test` (`#[test]`,
/// `#[tokio::test(...)]`, `#[cfg_attr(..., test)]`); `#[cfg(test)]` is not.
fn is_test_attr(attr: &str) -> bool {
    attr_paths(attr)
        .iter()
        .any(|p| p.rsplit("::").next() == Some("test"))
}

/// Attributes on the contiguous lines just before `i` (doc lines are skipped).
fn attrs_before(lines: &[&str], i: usize) -> Vec<String> {
    let mut attrs = Vec::new();
    let mut k = i;
    while k > 0 {
        let prev = lines[k - 1];
        if is_doc(prev) {
            k -= 1;
        } else if strip_comment(prev).ends_with(']') {
            // Walk back to the `#[` that opens this (possibly multi-line) attribute.
            let mut start = k - 1;
            while start > 0 && !is_attr_start(lines[start]) {
                start -= 1;
            }
            if !is_attr_start(lines[start]) {
                break;
            }
            attrs.push(
                lines[start..k]
                    .iter()
                    .map(|l| strip_comment(l))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            k = start;
        } else {
            break;
        }
    }
    attrs
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
        let body = body.trim();
        let Some(list) = body.strip_prefix("Cites:") else {
            if body.len() >= 6 && body[..6].eq_ignore_ascii_case("cites:") {
                out.problems.push(Problem {
                    file: file.to_owned(),
                    line: i + 1,
                    message: "citation marker must be spelled exactly `Cites:`".to_owned(),
                });
            }
            i += 1;
            continue;
        };
        let line_no = i + 1;

        // Collect the ID list, following `,`-terminated continuation lines.
        let mut list = list.trim().to_owned();
        let mut j = i + 1;
        let mut after_cites = j;
        while list.ends_with(',') && j < lines.len() {
            let Some(more) = lines[j].trim_start().strip_prefix("///") else {
                break;
            };
            list.push(' ');
            list.push_str(more.trim());
            j += 1;
            after_cites = j;
        }
        let ids: Vec<&str> = list
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .collect();

        // Walk forward to the item the doc comment is attached to.
        let mut attrs = attrs_before(&lines, i);
        let mut function = None;
        while j < lines.len() {
            let u = lines[j];
            if is_doc(u) || strip_comment(u).is_empty() {
                j += 1;
            } else if is_attr_start(u) {
                let (attr, rest, next) = take_attr(&lines, j);
                attrs.push(attr);
                j = next;
                if !rest.is_empty() {
                    function = fn_name(&rest);
                    break;
                }
            } else {
                function = fn_name(u);
                break;
            }
        }
        let is_test = attrs.iter().any(|a| is_test_attr(a));

        if ids.is_empty() {
            out.problems.push(Problem {
                file: file.to_owned(),
                line: line_no,
                message: "empty `Cites:` list".to_owned(),
            });
        }
        match (&function, is_test) {
            (None, _) => out.problems.push(Problem {
                file: file.to_owned(),
                line: line_no,
                message: "citation is not attached to a function".to_owned(),
            }),
            (Some(f), false) => out.problems.push(Problem {
                file: file.to_owned(),
                line: line_no,
                message: format!(
                    "citation is on `{f}`, which is not a test function (no `#[test]`-style attribute)"
                ),
            }),
            (Some(_), true) => {}
        }
        let counts = function.is_some() && is_test;
        for id in ids {
            if !is_id(id) {
                out.problems.push(Problem {
                    file: file.to_owned(),
                    line: line_no,
                    message: format!("malformed requirement ID `{id}` (expected PREFIX-n)"),
                });
            } else if counts {
                out.citations.push(Citation {
                    id: id.to_owned(),
                    file: file.to_owned(),
                    line: line_no,
                    function: function.clone(),
                });
            }
        }
        i = after_cites;
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

    fn run(text: &str) -> Scan {
        let mut scan = Scan::default();
        scan_text("t.rs", text, &mut scan);
        scan
    }

    #[test]
    fn attached_multiline_attribute_and_continuation() {
        let s = run(
            "/// Cites: A-1,\n///   A-2\n/// more docs\n#[tokio::test(\n  start_paused = true\n)]\nasync fn go() {}\n",
        );
        let ids: Vec<(&str, Option<&str>)> = s
            .citations
            .iter()
            .map(|c| (c.id.as_str(), c.function.as_deref()))
            .collect();
        assert_eq!(ids, [("A-1", Some("go")), ("A-2", Some("go"))]);
        assert!(s.problems.is_empty(), "{:?}", s.problems);
    }

    #[test]
    fn attribute_before_doc_comment_and_same_line_fn() {
        let s = run(
            "#[test]\n/// Cites: A-1\nfn a() {}\n\n/// Cites: A-2\n#[test] // trailing comment\nfn b() {}\n\n/// Cites: A-3\n#[test] fn c() {}\n",
        );
        assert_eq!(s.citations.len(), 3);
        assert!(s.problems.is_empty(), "{:?}", s.problems);
    }

    #[test]
    fn detached_non_test_empty_and_lowercase() {
        let s = run(
            "/// Cites: B-1\nstruct S;\n\n/// Cites: B-2\nfn helper() {}\n\n/// Cites:\n#[test]\nfn empty() {}\n\n/// cites: B-3\n#[test]\nfn lower() {}\n",
        );
        let msgs: Vec<&str> = s.problems.iter().map(|p| p.message.as_str()).collect();
        assert_eq!(msgs.len(), 4, "{msgs:?}");
        assert!(msgs[0].contains("not attached"));
        assert!(msgs[1].contains("not a test function"));
        assert!(msgs[2].contains("empty"));
        assert!(msgs[3].contains("spelled exactly"));
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

#[cfg(test)]
mod counting_tests {
    use super::*;

    #[test]
    fn only_test_attached_citations_count() {
        let mut scan = Scan::default();
        scan_text(
            "t.rs",
            "/// Cites: B-1\nstruct S;\n\n/// Cites: B-2\nfn helper() {}\n\n#[cfg_attr(feature = \"x\",\n  test)]\n/// Cites: B-3\nfn multi_before() {}\n",
            &mut scan,
        );
        let ids: Vec<&str> = scan.citations.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["B-3"]);
        assert_eq!(scan.problems.len(), 2, "{:?}", scan.problems);
    }
}

#[cfg(test)]
mod round2_tests {
    use super::*;

    fn run(text: &str) -> Scan {
        let mut scan = Scan::default();
        scan_text("t.rs", text, &mut scan);
        scan
    }

    #[test]
    fn cfg_test_and_doc_attributes_are_not_test_attributes() {
        let s = run(
            "#[cfg(test)]\n/// Cites: A-1\nfn helper() {}\n\n#[cfg(not(test))]\n/// Cites: A-2\npub fn prod() {}\n\n#[doc = \"a test\"]\n/// Cites: A-3\nfn documented() {}\n\n#[cfg_attr(feature = \"x\", ignore, test)]\n/// Cites: A-4\nfn conditional() {}\n\n#[rstest]\n/// Cites: A-5\nfn other_framework() {}\n",
        );
        let ids: Vec<&str> = s.citations.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["A-4"]);
        assert_eq!(s.problems.len(), 4, "{:?}", s.problems);
    }

    #[test]
    fn two_cites_lines_in_one_doc_block_both_count() {
        let s = run("/// Cites: A-1\n/// Cites: A-2,\n///   A-3\n#[test]\nfn t() {}\n");
        let ids: Vec<&str> = s.citations.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["A-1", "A-2", "A-3"]);
        assert!(s.problems.is_empty(), "{:?}", s.problems);
    }

    #[test]
    fn strings_and_brackets_inside_attributes() {
        let s = run(
            "/// Cites: A-1\n#[ignore = \"flaky, see https://x/y\"]\n#[test]\nfn a() {}\n\n/// Cites: A-2\n#[test] fn b() { let _ = [0u8; 1]; }\n",
        );
        let ids: Vec<&str> = s.citations.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["A-1", "A-2"]);
        assert!(s.problems.is_empty(), "{:?}", s.problems);
    }
}
