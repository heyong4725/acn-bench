# ADR-8 — Determinism lints: every ambient clock and entropy source, and unordered maps

**Status:** accepted (foundation CI/lab). **IDs affected:** CON-5, TRC-24.

## Context
CON-5(b) names `Instant::now()`, `SystemTime::now()`, `rand::thread_rng()` "and equivalents". The shipped lint list had only those three plus `tokio::time::Instant::now` and `thread::sleep`; CLAUDE.md claimed `fastrand` was banned when it was not, and `rand` 0.9 renamed `thread_rng` to `rng`. Separately, CON-5(c) and TRC-24 require byte-identical bundles, and the most common way Rust programs lose that property is iterating a `HashMap`, whose order is randomised per process.

## Decision
- `disallowed-methods` lists the equivalents explicitly: `rand::rng`, `rand::random`, `fastrand::Rng::new`, `getrandom::fill`, `uuid::Uuid::new_v4`, `chrono::{Utc,Local}::now`, `time::OffsetDateTime::now_utc`, and `RandomState::new`.
- `disallowed-types` bans `std::collections::HashMap` and `HashSet` across the workspace, with `disallowed_types = "deny"`. Substrate code uses `BTreeMap`/`BTreeSet`; a crate that needs hashing for speed adds an insertion-ordered or fixed-hasher map through an ADR and a local `#[allow]` with the reason.
- `acn_emu::clock` and `acn_emu::rng` are the only modules expected to carry `#[allow(clippy::disallowed_methods)]`, per CON-5(b).

## Consequences
The ban is coarse on purpose: it is cheaper to justify one exception than to audit every map for whether its order leaks into a bundle. Lints see only first-party code, so a dependency that iterates a `HashMap` into output is caught by the byte-identity acceptance test (TRC-24), not here.
