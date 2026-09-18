# Contributing to acn-bench

The working rules are in [`CLAUDE.md`](CLAUDE.md) and they apply to people and agents alike. The invariants are in [`specs/000-constitution.md`](specs/000-constitution.md). This page is the short version.

## Two tracks

- **`lab/`** — exploration. Any Rust, any dependency. Gates: `cargo fmt` and `cargo clippy` on your crate, plus a lab note in `docs/lab/`. See [`lab/README.md`](lab/README.md).
- **`crates/`, `specs/`, `hypotheses/`** — the substrate, where numbers become citable. Everything below applies.

If you are not sure which track you are on, it is lab.

## A substrate change

1. Pick the task in [`TASKS.md`](TASKS.md). Restate the requirement IDs you will satisfy.
2. Write the tests first. Each test function carries `/// Cites: CON-5, TRC-24`.
3. Implement. No `unsafe`; no `unwrap`/`expect`/`panic!` in library code; inject `Clock` and `Rng`; no `HashMap` (ADR-8).
4. Add the IDs you implemented to `trace-scope.toml`. Record every interpretation as `docs/decisions/ADR-<n>.md`.
5. Regenerate `docs/generated/` with `cargo xtask docs-inventory`, then run `tools/ci.sh` (it checks that the generated docs are current).
6. Open a PR with a conventional-commit title and the template filled in. One spec concern per PR.
7. Review. While the project has one maintainer, independent review is recommended, not required (CON-16): the maintainer merges once the gates are green, and may ask a separate agent session for a cross-review using the prompt in `TASKS.md`. With a second maintainer, someone other than the implementer reviews every PR.

## Labels that are enforced

`cargo xtask pr-check` runs in CI and fails a PR that

- touches `specs/` without the **`spec-change`** label, or
- touches the frozen set or `env-hash.json` without **`env-change`**, once the M0 gate is closed.

If a spec and a test disagree, stop and open a **spec-conflict** issue (there is a template). Do not fix either in the same PR.

## Useful commands

```bash
tools/ci.sh                          # the whole gate chain (CON-9)
cargo xtask trace-check              # every in-scope ID cited; every reference resolves
cargo xtask docs-inventory [--check] # docs/generated/
cargo xtask env-hash [--check|--write]
cargo xtask pr-check --base origin/main --labels spec-change
```

Every command prints one JSON object on stdout and exits 0 only when `"ok": true`.
