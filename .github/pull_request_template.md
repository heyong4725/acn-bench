<!-- Title: conventional commit, `type(scope): subject`. It becomes the squash-merge subject (CON-11). -->

## What and why

## Requirement IDs (CON-11)

| ID | What this PR does for it | Cited by (test) |
|---|---|---|
|  |  |  |

IDs added to `trace-scope.toml`:

## Risk class (CON-10)

- [ ] **A** — docs, tests, tools, xtask, scenarios/synthetic
- [ ] **B** — run-path crate: affected acceptance suites run (`cargo test -p acn-accept --test <poc>`)
- [ ] **C** — frozen set: label `env-change`, updated `env-hash.json`, adversarial review requested
- [ ] touches `specs/`: label `spec-change`, rationale and IDs added / changed / retired stated above (CON-14)

## Interpretations (CON-15)

ADRs written:

## Gates

- [ ] `tools/ci.sh` green locally
- [ ] `docs/generated/` regenerated (`cargo xtask docs-inventory`)
- [ ] For POC tasks: bundle `run_id` and `acn hyp verdict` output attached

## Review (CON-16)

Reviewer is not the implementer. Findings as PR comments; the reviewer does not push.
