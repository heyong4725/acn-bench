# ADR-7 — `lab/` is outside the Cargo workspace and outside the determinism lints

**Status:** accepted (foundation CI/lab). **IDs affected:** CON-6, CON-23.

## Context
CON-23 lets lab crates depend on anything and exempts them from CON-4..18. As shipped, the root manifest neither listed nor excluded `lab/`, so cargo rejected every crate created there ("current package believes it's in a workspace when it's not"), and clippy, which uses the nearest `clippy.toml` above a crate, applied the substrate's CON-5 bans to lab code. The first lab spike (a QUIC timing prototype) needs `Instant::now()`.

## Decision
- The root manifest carries `exclude = ["lab"]`. Each lab crate is standalone: its own `Cargo.lock`, its own `target/`, any dependency it likes, none of it in the substrate's lockfile or `cargo deny` graph.
- `lab/clippy.toml` exists and carries no `disallowed-*` entries, shielding lab crates from the root configuration.
- Lab gates address the crate by manifest path (`cargo fmt|clippy --manifest-path lab/<slug>/Cargo.toml`). `target/` is ignored at any depth.
- This is not a substrate layout change under CON-6: no `crates/` member is added or moved.

## Consequences
A lab crate cannot use `workspace = true` inheritance; it states its own edition and dependency versions, which also keeps exploratory dependencies out of the reproducibility story. Graduation (CON-24) moves code into `crates/` under a `spec-change` PR, at which point the full lint posture applies.
