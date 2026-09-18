# ADR-3 — What `trace-check` counts as implemented, and how citations are parsed

**Status:** accepted (T01). **IDs affected:** CON-12, CON-23.

## Context
CON-12 says `cargo xtask trace-check` MUST fail when "a MUST in an implemented spec section has no citing test", but nothing in the specs marks a section as implemented, and specs are read-only for agents (CON-14). The check also needs unambiguous rules for what a `/// Cites:` comment is and where it may sit.

## Decision
1. **Scope file.** `trace-scope.toml` at the workspace root lists, per spec, the `sections` (by `## N.` heading number) and/or individual `ids` that are implemented. The PR that implements a MUST adds it to the scope. IDs outside the scope are still valid citation targets, so tests may cite ahead of full implementation; only in-scope MUSTs are required to be cited. Unknown IDs and empty sections in the scope file are errors.
2. **Requirement IDs** are paragraphs in `specs/<NNN>-*.md` that begin with a bold ID (`**CON-5**`). The level is the strongest RFC 2119 keyword in the paragraph (`MUST NOT` counts as MUST). Fenced code blocks are ignored. Duplicate definitions are an error.
3. **Citations** are `/// Cites: ID[, ID…]` doc-comment lines (a line ending in `,` continues on the next doc line) on a `fn` carrying a test attribute, meaning an attribute whose path ends in `test` (`#[test]`, `#[tokio::test(...)]`, `#[cfg_attr(..., test)]`); `#[cfg(test)]`, `#[doc]` and third-party runners such as `#[rstest]` do not qualify. A citation on a non-test function, on anything that is not a function, a malformed ID, a misspelled marker, or an empty list fails the check. See the amendment below for how sources are read.

## Consequences
Coverage is declared, not inferred: a reviewer can see in one file which MUSTs a PR claims. `docs/generated/requirements.md` renders the same model, so drift between spec, scope and tests is visible on every PR. Section-level scope lets a spec task claim a whole section at once; ID-level scope handles constitution clauses that are implemented piecemeal.

## Known limits of the spec grammar
The parser recognises an ID only when its bold form starts a line; bold IDs mid-line or inside tables are references, not definitions. Only `## N.` headings set the section (`### N.M` does not). A `MUST` inside backticks or a code fence counts as prose in the first case and is ignored in the second; spec authors keep normative keywords out of code spans. A root with no `specs/<NNN>-*.md` files is an error, not an empty pass. Duplicate ID definitions, within or across files, are an error.

## Amendment (pre-landing review of T01) — sources are parsed, and only compiled code counts
The first scanner read Rust as lines of text. Three independent reviews each found a different way to make it count a test that does not exist: a test inside a block comment or a string literal, a `cfg_attr` predicate or a string that merely mentions `test`, a `#[test]` borrowed from an earlier function, a file that no `mod` line names, a file under the virtual workspace root's `tests/`, and a file that is not Rust at all. Each patch invited the next, so the scanner now parses with `syn` and walks what cargo compiles:
- It reads the root `Cargo.toml` for the workspace members, takes each member's crate roots (`src/lib.rs`, `src/main.rs`, `src/bin/*.rs`, `[lib]`/`[[bin]]`/`[[test]]` paths, and top-level `tests/*.rs` unless `autotests = false`), and follows `mod` declarations, including `#[path]`. A file nothing declares is never read. `lab/` is not a member and is never scanned (CON-23).
- A file that does not parse is a problem and contributes nothing. Doc comments are read as the attributes they desugar to, so comments, strings and macro bodies cannot carry a citation, and a test attribute is judged from its parsed path.
- A root with no `Cargo.toml`, and a root with no `trace-scope.toml`, are errors: losing either file must not turn the gate green.
- An `#[ignore]`d test still counts. The `live` and `netem` tiers are `#[ignore]` by design (CON-12); whether those tiers ran is a CI question, not a citation question.
- Known limit: code under a false `cfg` (for example `#[cfg(any())]`) is parsed and would count. It does not compile into any test binary, so a reviewer should treat it like commented-out code.

The spec parser was tightened in the same review: a code fence closes only on the same character with at least the opening length (so a longer fence cannot swallow the rest of a spec, and `~~~` is recognised), an unclosed fence is an error, a requirement may sit behind a list marker, and a leading byte-order mark is ignored.
