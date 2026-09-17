# ADR-4 — `env-hash` covers exactly the frozen set and is recorded at the root

**Status:** accepted (T01). **IDs affected:** CON-5(e), CON-7, CON-9.

## Context
CON-7 requires an `env-change` PR to carry "the updated output of `cargo xtask env-hash`" and CON-9 gates on `env-hash --check`, but neither says what the hash covers nor where the reference value lives. CON-5(e) later folds `env_hash` into `run_id`, so the definition must be stable and machine-readable.

## Decision
- **Coverage** is the frozen set as listed in CON-7, and nothing else: `hypotheses/`, `scenarios/measured/`, `crates/acn-hyp/`, `crates/acn-attrib/src/core/`, `crates/acn-trace/src/schema/`. Placeholder files (`.gitkeep`, `.DS_Store`) and `target/` are excluded. The toolchain pin and `Cargo.lock` are deliberately not included; if reproducibility work later needs them in `run_id`, that is a separate, named input rather than a widening of the frozen-set hash.
- **Definition.** For each file, `blake3(contents)`; the environment hash is `blake3` over the sorted sequence of `path \0 file_hash \n` records, paths relative to the root with forward slashes. Platform-independent and order-independent of the filesystem.
- **Record.** `env-hash.json` at the workspace root holds the hash and the per-file list, written only by `cargo xtask env-hash --write`. It is added to CODEOWNERS next to the frozen paths so it cannot change without the Class C owner.

## Consequences
A change to any frozen file fails `env-hash --check` until the record is rewritten in the same PR, which is the mechanical hook for the `env-change` process. Adding a file to a frozen directory also moves the hash. Run-time consumers (`acn-cli`, T02+) read `env-hash.json` rather than recomputing, so a bundle's `env_hash` is the value that was reviewed.
