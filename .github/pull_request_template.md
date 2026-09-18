*Title: conventional commit, `type(scope): subject`. It becomes the squash-merge subject (CON-11). Delete this line.*

## What and why

## Requirement IDs (CON-11)

| ID | What this PR does for it | Cited by (test) |
|---|---|---|
|  |  |  |

IDs added to `trace-scope.toml`:

## Risk class (CON-10)

A PR that touches more than one class is reviewed at the highest. One spec concern per PR (CON-11).

- [ ] **A** — docs, tests, tools, xtask, scenarios/synthetic
- [ ] **B** — run-path crate: affected acceptance suites run (`cargo test -p acn-accept --test <poc>`)
- [ ] **C** — frozen set: updated `env-hash.json`; after the M0 gate also label `env-change`, adversarial review done and linked, **human-merged** (CON-7)
- [ ] touches `specs/` or removes an entry from `trace-scope.toml`: label `spec-change`, rationale and IDs added / changed / retired stated above (CON-14)
- [ ] touches an enforcement point (workflows, `clippy.toml`, `deny.toml`, `.cargo/`, root manifest or lockfile, `tools/ci.sh`, `crates/xtask/`, CODEOWNERS): said so above, and the copy in `crates/xtask/tests/workspace.rs` updated

## Interpretations (CON-15)

ADRs written:

## Gates

- [ ] `docs/generated/` regenerated (`cargo xtask docs-inventory`)
- [ ] `tools/ci.sh` green locally (documentation-only changes may run `fmt`, `docs-inventory --check` and `trace-check` only, CON-9)
- [ ] For POC tasks (from T05 on): bundle `run_id` and `acn hyp verdict` output attached

## Review (CON-16)

- [ ] Solo-maintainer mode: gates green; cross-review by a separate session is recommended for risky Class B/C changes (link it if done)
- [ ] With a second maintainer: reviewed by someone other than the implementer

Findings go in PR comments; a reviewer does not push.
