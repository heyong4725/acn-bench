# ADR-3 — What `trace-check` counts as implemented, and how citations are parsed

**Status:** accepted (T01). **IDs affected:** CON-12, CON-23.

## Context
CON-12 says `cargo xtask trace-check` MUST fail when "a MUST in an implemented spec section has no citing test", but nothing in the specs marks a section as implemented, and specs are read-only for agents (CON-14). The check also needs unambiguous rules for what a `/// Cites:` comment is and where it may sit.

## Decision
1. **Scope file.** `trace-scope.toml` at the workspace root lists, per spec, the `sections` (by `## N.` heading number) and/or individual `ids` that are implemented. The PR that implements a MUST adds it to the scope. IDs outside the scope are still valid citation targets, so tests may cite ahead of full implementation; only in-scope MUSTs are required to be cited. Unknown IDs and empty sections in the scope file are errors.
2. **Requirement IDs** are paragraphs in `specs/<NNN>-*.md` that begin with a bold ID (`**CON-5**`). The level is the strongest RFC 2119 keyword in the paragraph (`MUST NOT` counts as MUST). Fenced code blocks are ignored. Duplicate definitions are an error.
3. **Citations** are `/// Cites: ID[, ID…]` doc-comment lines in `.rs` files under `crates/` and `tests/`, which must be attached (through further doc lines and attributes) to a `fn`. A citation on anything else, a malformed ID, or an empty list fails the check. `lab/` is not scanned (CON-23). Directories named `target` or `fixtures` are skipped so self-hosting fixtures can contain deliberately broken citations.

## Consequences
Coverage is declared, not inferred: a reviewer can see in one file which MUSTs a PR claims. `docs/generated/requirements.md` renders the same model, so drift between spec, scope and tests is visible on every PR. Section-level scope lets a spec task claim a whole section at once; ID-level scope handles constitution clauses that are implemented piecemeal.
