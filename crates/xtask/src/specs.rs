//! Requirement-ID extraction from `specs/<NNN>-*.md` (CON-12).
//!
//! A requirement is a paragraph that starts with a bold ID such as `**CON-5**`.
//! Its level is the strongest RFC 2119 keyword in the paragraph. IDs inside
//! fenced code blocks are ignored, and only the `## N.` heading in force gives
//! the section.

use std::path::Path;

use serde::Serialize;

use crate::workspace::read;
use crate::{Error, Result};

/// RFC 2119 level of a requirement paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Level {
    Must,
    Should,
    May,
    None,
}

/// One requirement ID as defined in a spec.
#[derive(Debug, Clone, Serialize)]
pub struct Requirement {
    pub id: String,
    pub prefix: String,
    pub number: u32,
    /// Spec number, e.g. `"000"`.
    pub spec: String,
    /// Spec file name, e.g. `"000-constitution.md"`.
    pub file: String,
    /// Section token from the enclosing `## N.` heading, if any.
    pub section: Option<String>,
    pub line: usize,
    pub level: Level,
}

/// Whether `token` has the shape of a requirement ID: `PREFIX-n`.
pub fn is_id(token: &str) -> bool {
    let Some((prefix, number)) = token.split_once('-') else {
        return false;
    };
    let prefix_ok = prefix
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase())
        && prefix
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    let number_ok = !number.is_empty() && number.chars().all(|c| c.is_ascii_digit());
    prefix_ok && number_ok
}

/// Split an ID into prefix and number; `None` if it is not an ID.
pub fn split_id(token: &str) -> Option<(&str, u32)> {
    if !is_id(token) {
        return None;
    }
    let (prefix, number) = token.split_once('-')?;
    Some((prefix, number.parse().ok()?))
}

/// The spec number of a file name like `010-trace-schema.md`.
pub fn spec_number(file_name: &str) -> Option<&str> {
    let (num, _) = file_name.split_once('-')?;
    (!num.is_empty() && num.chars().all(|c| c.is_ascii_digit())).then_some(num)
}

fn level_of(paragraph: &str) -> Level {
    let mut level = Level::None;
    for word in paragraph.split(|c: char| !c.is_ascii_alphabetic()) {
        match word {
            "MUST" => return Level::Must,
            "SHOULD" => level = Level::Should,
            "MAY" if level == Level::None => level = Level::May,
            _ => {}
        }
    }
    level
}

fn section_token(heading: &str) -> Option<String> {
    let token: String = heading
        .trim()
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != '.')
        .collect();
    (!token.is_empty() && token.chars().next().is_some_and(|c| c.is_ascii_digit())).then_some(token)
}

/// Parse one spec's text. Pure, for unit tests.
pub fn parse_spec_text(file: &str, spec: &str, text: &str) -> Vec<Requirement> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut in_fence = false;
    let mut section: Option<String> = None;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim_start();
        if t.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some(h) = t.strip_prefix("## ") {
            section = section_token(h);
            continue;
        }
        let Some(rest) = t.strip_prefix("**") else {
            continue;
        };
        let Some(end) = rest.find("**") else {
            continue;
        };
        let token = &rest[..end];
        let Some((prefix, number)) = split_id(token) else {
            continue;
        };
        if out.iter().any(|r: &Requirement| r.id == token) {
            continue;
        }
        let mut paragraph = (*line).to_owned();
        for l in &lines[i + 1..] {
            if l.trim().is_empty() {
                break;
            }
            paragraph.push(' ');
            paragraph.push_str(l);
        }
        out.push(Requirement {
            id: token.to_owned(),
            prefix: prefix.to_owned(),
            number,
            spec: spec.to_owned(),
            file: file.to_owned(),
            section: section.clone(),
            line: i + 1,
            level: level_of(&paragraph),
        });
    }
    out
}

/// Spec files under `<root>/specs/` that carry a numeric prefix, sorted.
pub fn spec_files(root: &Path) -> Result<Vec<(String, String)>> {
    let dir = root.join("specs");
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| Error::io(&dir, e))? {
        let entry = entry.map_err(|e| Error::io(&dir, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.ends_with(".md") {
            continue;
        }
        if let Some(num) = spec_number(&name) {
            files.push((num.to_owned(), name));
        }
    }
    files.sort();
    Ok(files)
}

/// All requirements defined under `<root>/specs/`, in file then line order.
pub fn parse_specs(root: &Path) -> Result<Vec<Requirement>> {
    let mut out = Vec::new();
    for (num, name) in spec_files(root)? {
        let text = read(&root.join("specs").join(&name))?;
        let reqs = parse_spec_text(&name, &num, &text);
        for r in reqs {
            if let Some(dup) = out.iter().find(|x: &&Requirement| x.id == r.id) {
                return Err(Error::Invalid(format!(
                    "requirement {} is defined twice: {}:{} and {}:{}",
                    r.id, dup.file, dup.line, r.file, r.line
                )));
            }
            out.push(r);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_shape() {
        assert!(is_id("CON-5"));
        assert!(is_id("P1A-12"));
        assert!(!is_id("con-5"));
        assert!(!is_id("CON-"));
        assert!(!is_id("Status:"));
        assert!(!is_id("L0-Build"));
    }

    #[test]
    fn levels_and_sections() {
        let text = "## 2. Rules\n\n**X-1** It MUST NOT fail.\n\n**X-2** It MAY\ncontinue and SHOULD end.\n\n```\n**X-3** fenced MUST\n```\n\n**X-4** plain text.\n";
        let reqs = parse_spec_text("900-x.md", "900", text);
        let ids: Vec<&str> = reqs.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["X-1", "X-2", "X-4"]);
        assert_eq!(reqs[0].level, Level::Must);
        assert_eq!(reqs[1].level, Level::Should);
        assert_eq!(reqs[2].level, Level::None);
        assert_eq!(reqs[0].section.as_deref(), Some("2"));
    }

    #[test]
    fn spec_numbers() {
        assert_eq!(spec_number("000-constitution.md"), Some("000"));
        assert_eq!(spec_number("README.md"), None);
    }
}
