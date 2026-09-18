# ADR-6 — `cargo xtask pr-check`: label rules and CODEOWNERS coverage as machine checks

**Status:** accepted (xtask hardening). **IDs affected:** CON-7, CON-14, LOOP-20.

## Context
CON-14 (spec edits need the `spec-change` label), CON-7 (frozen-set edits need `env-change`) and LOOP-20 (CI verifies that CODEOWNERS covers the protected paths) were rules on paper: no check failed when they were broken. The constitution also says the `env-change` procedure applies "after the M0 gate", while T02–T05 legitimately populate frozen directories before it.

## Decision
- `cargo xtask pr-check [--base <ref> | --changed a,b,…] [--labels x,y]` prints one CON-8 object. With a change list it fails when a path under `specs/` changed without the `spec-change` label, and when a frozen-set path or `env-hash.json` changed without `env-change`.
- The `env-change` rule is enforced only once `docs/gates/M0.md` exists. In git mode the record is looked up on the merge base as well as on the head, so a PR cannot reopen the frozen set by deleting the gate record in the same change. Before that the same finding is returned under `advisories` and does not fail, so bootstrap tasks are not blocked and nobody learns to apply the label reflexively.
- With or without a change list, the task fails unless `.github/CODEOWNERS` protects each frozen path, `/specs/`, `/env-hash.json`, `/trace-scope.toml`, `/docs/gates/` and `/.github/CODEOWNERS` itself. GitHub applies the **last** matching pattern, so a path counts as protected only when the last entry that could match it (a catch-all, a parent, a child glob or the path itself) is the exact path with a real owner (`@user`, `@org/team` or an e-mail). Globs and parent directories never count as coverage, and the overlap test over-approximates gitignore matching on purpose: an unanchored pattern (`mod.rs`, `core/`, `*.md`) can match at any depth and so overlaps every protected path, and an anchored glob overlaps whenever its literal prefix and the protected path share a string prefix (`/spec*`). The check fails closed; keep the exact protected entries last in the file.
- Changed paths come from `git diff --no-renames --name-only -z`, so both sides of a move are seen and non-ASCII paths are not quoted; explicit paths are normalised lexically (`./`, `..`) and matched case-insensitively, because the primary platform's filesystem is. A `--base` beginning with `-` is refused. The list is derived from the same `FROZEN_SET` constant that `env-hash` uses, so the two cannot drift.
- The CON-9 gate chain is fixed by the constitution and is not extended. The static half runs inside `cargo test` through a self-hosting test; the label half runs in CI as its own job, which has the PR's labels and base ref.

## Consequences
A mislabelled PR fails a required check instead of relying on reviewer memory. The check is advisory about who merges: GitHub branch protection still has to require code-owner review for CODEOWNERS to bind, which is a repository setting, not code.
