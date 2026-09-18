# ADR-4 — `env-hash` covers exactly the frozen set and is recorded at the root

**Status:** accepted (T01). **IDs affected:** CON-5(e), CON-7, CON-9.

## Context
CON-7 requires an `env-change` PR to carry "the updated output of `cargo xtask env-hash`" and CON-9 gates on `env-hash --check`, but neither says what the hash covers nor where the reference value lives. CON-5(e) later folds `env_hash` into `run_id`, so the definition must be stable and machine-readable.

## Decision
- **Coverage** is the frozen set as listed in CON-7, and nothing else: `hypotheses/`, `scenarios/measured/`, `crates/acn-hyp/`, `crates/acn-attrib/src/core/`, `crates/acn-trace/src/schema/`. Nothing is skipped by name except a zero-length `.gitkeep`: a placeholder with content is content (`#[path]` and `include!` can turn any file name into code), `.DS_Store` is refused with a message saying how to remove it, and symlinks and special files are refused because their targets would be hashed by name only. Files are hashed from the working tree as it is, so an editor swap file or a build directory inside a frozen directory changes the hash; keep frozen directories clean. The toolchain pin and `Cargo.lock` are deliberately not included; if reproducibility work later needs them in `run_id`, that is a separate, named input rather than a widening of the frozen-set hash.
- **Definition.** For each file, `blake3(contents)`; the environment hash is `blake3` over the sorted sequence of `path \0 file_hash \n` records, paths relative to the root with forward slashes. Platform-independent and order-independent of the filesystem.
- **Record.** `env-hash.json` at the workspace root holds the hash and the per-file list, written only by `cargo xtask env-hash --write`. It is added to CODEOWNERS next to the frozen paths so it cannot change without the Class C owner.

## Consequences
A change to any frozen file fails `env-hash --check` until the record is rewritten in the same PR, which is the mechanical hook for the `env-change` process. Adding a file to a frozen directory also moves the hash. Run-time consumers (`acn-cli`, T02+) read `env-hash.json` rather than recomputing, so a bundle's `env_hash` is the value that was reviewed.

## Amendment (pre-landing review of T01) — `--check` verifies the whole record
`--check` first compared only the top-level `env_hash`. A one-line edit to that value would pass while every per-file hash, which is what a reviewer reads to see which frozen file moved, still said nothing had changed. The check now requires the recorded `files` list to hash to the recorded `env_hash` and the whole record to equal the computed one. A missing frozen directory or a `--root` that is not a directory is an error, never an empty set, and `--write` does not read the old record, so it can repair a corrupt one.
