# SPEC 000 — Constitution

**Status:** Draft v0.2 (September 2026; v0.2 adds canonical run-ID, seed and build-hash encodings as CON-27, the RNG pin in CON-5, normative wording for CON-6/9/10, and the scope file in CON-12). **Inherits:** none. **Owner:** Young He.
**Scope:** invariants that every other spec, crate, test and PR in acn-bench inherits. The key words MUST, MUST NOT, SHOULD, MAY are as in RFC 2119. Requirement IDs `CON-n` are stable once published; retired IDs are never reused.

## 0. Project definition

acn-bench is an independent Rust project that (a) builds the experimental substrate described in the ACN Technical Report v1.2, Appendix E §E.2 — a deterministic network simulator/emulator, an agent-workload generator, an environment replayer and a control-and-evidence plane — and (b) runs the report's proof-of-concept catalogue (POC 1a–16) as frozen, falsifiable hypotheses with mandatory controls. It borrows the *method* of spec-driven development and testing from prior work and nothing else. ACN is an exploration of a new field: the POC catalogue is the current map, not the boundary, and this constitution deliberately separates an **exploration track** (`lab/`, §7) where ideas are cheap from a **substrate track** (`crates/`, `hypotheses/`) where results are defensible. Rigor is something an idea graduates into, not a precondition for trying it.

## 1. Platform and toolchain

**CON-1** The primary development platform is macOS arm64. All crates MUST build and pass `unit`, `integ` and `accept` tiers on macOS arm64 and on Linux x86_64/aarch64. Linux-only functionality MUST be isolated behind `#[cfg(target_os = "linux")]` and the cargo feature `netem`, and MUST NOT be required by any default build or test.

**CON-2** The implementation language is Rust, stable channel, pinned in `rust-toolchain.toml`, edition 2024. One Cargo workspace; crates under `crates/`. Python, notebooks, or any other language MUST NOT appear on the run path or the analysis path; shell is permitted only in `tools/*.sh`. Exceptions require an ADR (CON-15) naming the boundary and the reason.

**CON-3** The async runtime is tokio. No other executor MAY be introduced. Blocking work on the run path MUST go through `tokio::task::spawn_blocking`.

**CON-4** All trace and measurement data MUST be Apache Arrow in memory and Parquet on disk, under the schema of SPEC 010. JSON is permitted only for CLI results (CON-8), run manifests, hypothesis verdicts and control-plane messages (SPEC 070). CSV MUST NOT be produced or consumed on the run path.

## 2. Determinism and reproducibility

**CON-5** Determinism is a feature. (a) Every source of randomness MUST be an injected `ChaCha20Rng` seeded from the run seed, with per-component sub-streams derived by name as CON-27 specifies. The generator crate and every crate that turns its output into samples (`rand_chacha`, `rand_core`, `rand`, `rand_distr` where used) MUST be pinned exactly (`=x.y.z`) in the workspace manifest, and a golden-vector test MUST assert the first raw outputs of a named sub-stream, one ranged integer and one distribution sample; changing any of those crates or versions is a Class C change, because it silently invalidates every earlier run. (b′) Trace and span identifiers in `sim` mode MUST come from a seeded `IdGenerator` (SPEC 010, TRC-27). (b) Every source of time MUST be an injected `Clock`; `Instant::now()`, `SystemTime::now()`, `rand::thread_rng()` and equivalents MUST NOT be called outside `acn_emu::clock` and `acn_emu::rng`, enforced by clippy `disallowed_methods`. (c) In `sim` mode, the same (seed, scenario, workload, hypothesis, environment, build) tuple MUST produce a byte-identical bundle, where *build* is the `build_hash` of CON-27 and the other members are the inputs of `run_id`; nothing that can affect the bytes of a sim bundle MAY come from anywhere but the members of this tuple (a command-line option that does is an `opt.` parameter, CON-27b). (d) In `live` mode the impairment schedule MUST be precomputed from the seed before traffic starts, and outcomes are reported as distributions with replicate counts, never single runs. (e) Every bundle MUST carry a `run_id` derived from the seed, the scenario, workload, hypothesis and environment hashes, the mode and the run parameters, exactly as CON-27(b) specifies, and MUST be re-derivable from those inputs.

**CON-6** The substrate layout MUST be: `specs/`, `hypotheses/`, `scenarios/{synthetic,measured}/`, `crates/{acn-trace,acn-emu,acn-mockllm,acn-harness,acn-gen,acn-replay,acn-ctl,acn-hyp,acn-attrib,acn-cli,xtask}/`, `tests/accept/` (workspace package `acn-accept`), `tools/`, `docs/{decisions,gates,generated,lab}/`, `runs/` (gitignored). Adding a crate under `crates/` or changing the substrate layout requires a `spec-change` PR. `lab/` (§7) and other top-level directories MAY be added freely.

**CON-7** The frozen set is: `hypotheses/` (frozen-status files only, see CON-17), `scenarios/measured/`, `crates/acn-hyp/`, `crates/acn-attrib/src/core/`, `crates/acn-trace/src/schema/`. After the M0 gate, any change to the frozen set MUST be a human-merged PR labelled `env-change` carrying the updated output of `cargo xtask env-hash`, and MUST receive an adversarial review whose stated purpose is to break the integrity story. Automated agents and auto-research loops MUST NOT hold write access to the frozen set.

**CON-27** Canonical encodings. Every hash is plain BLAKE3 (`blake3::hash`, not keyed or `derive_key` mode) with a 32-byte digest, printed as lowercase hex. In every preimage below, a digest is its raw 32 bytes, `seed` is an unsigned 64-bit little-endian integer, a *string* is its UTF-8 bytes preceded by its length as an unsigned 32-bit little-endian integer, and the preimage MUST begin with the stated ASCII context string terminated by one zero byte, so that no two derivations can collide. `env_hash` alone keeps the un-prefixed record construction of ADR-4. (a) **File and tree hashes.** `hypothesis_hash`, `scenario_hash` and `workload_hash` are `blake3` of the defining file's bytes as held in the git blob; the repository MUST carry a `.gitattributes` that marks every hashed input (`hypotheses/**`, `lab/hypotheses/**`, `scenarios/**`, and workload definitions) `-text`, so that checkout settings cannot change them. An input that is more than one file is `blake3("acn-bench/tree/v1\0" ‖ records)` over the sorted `path \0 file_hash \n` records of ADR-4, and a scenario or workload file MUST reference any other file it reads by that file's BLAKE3, as TRC-40 does. A run with no hypothesis (fixtures, lab runs, LOOP-21 researcher sessions) uses 32 zero bytes, with hypothesis id `none` and status `candidate`. A run MUST recompute `env_hash` and MUST refuse to start when it differs from `env-hash.json`. (b) **Run identity.** `run_id = blake3("acn-bench/run_id/v1\0" ‖ seed ‖ scenario_hash ‖ workload_hash ‖ hypothesis_hash ‖ env_hash ‖ mode ‖ params_hash)`, where `mode` is one byte (0 `sim`, 1 `live`, 2 `netem`) and `params_hash = blake3("acn-bench/params/v1\0" ‖ pairs)` over the run's parameters as key/value strings sorted by key: `backend`, `arms` (sorted, comma-joined), `replicates` (decimal), one `vary.<name>` per `[varies]` parameter of the cell (SPEC 080), and one `opt.<name>` per run option that can affect bundle bytes. A sim run, its live twin (CON-25), a mock run and a real-provider run of the same cell therefore have four different run IDs. (c) **Random sub-streams.** The seed of the component sub-stream `name` (ASCII) MUST be the 32 bytes `blake3("acn-bench/rng/v1\0" ‖ replicate_seed ‖ name)` fed to `ChaCha20Rng::from_seed`, with `name` as a string and last; adding a component therefore never shifts another component's stream. (d) **Replicates.** `replicate_seed(i)` MUST be the first 8 bytes, little-endian, of `blake3("acn-bench/replicate/v1\0" ‖ seed ‖ i)`, with `i` as an unsigned 32-bit little-endian integer. `acn.seed` (TRC-10) records the run seed and `acn.replicate` records `i`. (e) **Build identity.** `build_hash = blake3("acn-bench/build/v1\0" ‖ blake3(Cargo.lock) ‖ blake3(rust-toolchain.toml) ‖ source_hash ‖ target ‖ profile ‖ features)`, where `source_hash` is the tree hash of (a) over every tracked file under `crates/` plus the root `Cargo.toml`, and `target` (the target triple), `profile` and `features` (sorted, comma-joined) are strings. It MUST be computed at compile time and embedded in the binary, and every manifest MUST carry it. It is deliberately not an input of `run_id`, which identifies the experiment rather than the binary. Byte-identity under CON-5(c) is claimed only between bundles whose `build_hash` is equal, and so only within one target triple; sim-path code SHOULD take transcendental functions from the `libm` crate rather than the platform's, so that identity across targets stays attainable, but it is not claimed. A change to any encoding in this clause is a Class C change and takes a new context version (`/v2`).


## 3. Interfaces

**CON-8** Every CLI entry point (`acn <subcommand>`, `cargo xtask <task>`) MUST print exactly one JSON object to stdout, MUST write logs only to stderr via `tracing`, and MUST exit 0 if and only if the object contains `"ok": true`. The object MUST include `"run_id"` when a run was created or read.

**CON-9** Pre-commit gates MUST be run in this order, chained with `&&`: `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace`; `cargo xtask trace-check`; `cargo xtask docs-inventory --check`; `cargo xtask env-hash --check`; `cargo deny check`. Class B changes add the affected acceptance suites; `netem` changes add the Linux ignored tier. Documentation-only changes MAY run only `fmt`, `docs-inventory` and `trace-check`.

**CON-10** Risk classes: **A** — `docs/`, `tests/`, `tools/`, `xtask`, `scenarios/synthetic/` (baseline gates). **B** — run-path crates (baseline gates plus affected acceptance suites). **C** — the frozen set (CON-7 procedure). A PR touching more than one class MUST be reviewed at the highest class it touches.

**CON-11** Commits follow Conventional Commits (`feat`, `fix`, `test`, `spec`, `docs`, `refactor`, `chore`, `ci`, `perf`) with a scope naming the crate or spec. One spec concern per PR. The PR description MUST list the requirement IDs satisfied. The repository squash-merges; the PR title is the mainline commit subject.

## 4. Tests as the spec's teeth

**CON-12** Test tiers are `unit`, `integ`, `accept`, `live`, `netem` as defined in CLAUDE.md. Every implemented MUST MUST be cited by at least one test function through a `/// Cites: <ID>[, <ID>…]` doc comment; which sections and IDs are implemented is declared in `trace-scope.toml`, and `cargo xtask trace-check` MUST fail when any requirement ID declared there has no citing test function, whatever its wording; MUST fail when a citation names an ID that does not exist; and MUST fail when a document or frozen hypothesis file refers to an ID that its written spec does not define (a reference to a spec that `specs/README.md` lists but that is not yet written is reported as a forward reference and does not fail). Acceptance tests for a POC MUST cite that POC's spec IDs and the hypothesis file it instantiates.

**CON-13** When a spec and a test disagree, the implementer MUST stop and open an issue titled `spec-conflict: <ids>`; neither the spec nor the test is changed in the same PR.

**CON-14** Agents MUST NOT edit `specs/` except in a PR labelled `spec-change` that states the rationale and the IDs added, changed or retired.

**CON-15** When a spec is ambiguous, the implementer MUST record the interpretation in `docs/decisions/ADR-<n>.md` (context, decision, consequences, IDs affected) and proceed. Work MUST NOT stall on ambiguity.

**CON-16** Implementation and review of a PR MUST be performed by different agents (or a human reviewer). Review findings are PR comments; the reviewer MUST NOT push to the branch.

## 5. Experimental integrity

**CON-17** A hypothesis file `<poc>.toml` (SPEC 080) states the hypothesis, the variables a loop may vary and their ranges, the measured quantities, the control, the falsifier as a machine-checkable predicate over the bundle, and the expected outcome. Hypotheses have a lifecycle: `candidate` files live in `lab/hypotheses/`, MAY be edited by anyone including automated loops, and produce verdicts labelled *exploratory*; a hypothesis becomes `frozen` by moving it to `hypotheses/` in a Class C PR, which MUST happen before the first run whose bundle is cited in a report, a paper or a working-group contribution. `acn hyp verdict` is the only path from a bundle to a pass / fail / inconclusive verdict, and it MUST label the verdict with the hypothesis status.

**CON-18** Every POC acceptance suite MUST run the control named in its hypothesis file under the same scenario as the treatment, and MUST report the treatment-minus-control effect with replicate count and confidence interval. A suite without its control is incomplete and MUST NOT close a gate.

**CON-19** `#![forbid(unsafe_code)]` in every crate. Library code MUST return `Result` and MUST NOT `unwrap`, `expect` or `panic!` except in tests and in `main` after error formatting; clippy `unwrap_used`, `expect_used` and `panic` lints are denied in library targets.

**CON-20** The substrate and every frozen-hypothesis result MUST run without `dora-rs`, `aisle`, Genesis, ROS or any robotics/dataflow framework: none of these MAY be a required dependency of a `crates/` member. Optional adapters (a partner's dora or ROS 2 stack, a specific radio testbed) MAY exist as feature-gated or `lab/` crates that attach at the control plane (SPEC 070) and the trace schema (SPEC 010). Ideas from other projects are welcome and are re-specified here when they graduate; source code is not copied.

**CON-21** Measured impairment traces under `scenarios/measured/` MUST carry a provenance file (collection date, device, location class, method, licence) and MUST NOT contain payload data or identifiers; only timing, size and link-state fields as defined in SPEC 020.

## 6. Milestone gates

**CON-22** Milestones M0–M4 are defined in SPEC 095. A gate closes only by a human-merged PR that adds `docs/gates/M<n>.md` recording the evidence the gate spec requires, including the run IDs of the bundles cited. No task marked as post-gate in `TASKS.md` MAY start before the gate closes.

**CON-25** Sim ↔ live twin. Any `sim` result that is cited MUST have a `live` run of the same scenario and seed schedule in the same bundle set; the per-quantity divergence MUST be recorded in the bundle and MUST be within the tolerance declared in the hypothesis file, otherwise the verdict is `inconclusive`. From M4 the same rule applies between `live` and `netem` on measured traces.

**CON-26** Mock backends are models, not results. Bundles whose inference backend is `acn-mockllm` MUST carry `backend = "mockllm"` and MUST NOT be cited as results; `acn hyp verdict` on such a bundle MUST label its output *mock-gated*. Hypotheses that concern provider behaviour (caching, streaming, rate limiting) MUST be instantiated and reported per provider.

## 7. Exploration track

**CON-23** `lab/` is the exploration track. Anything under `lab/` is Rust (CON-2 still applies) but is exempt from CON-4 through CON-18 except CON-8 for anything that becomes a CLI. The only gates for `lab/` are `cargo fmt` and `cargo clippy` on the lab crate itself, and a lab note in `docs/lab/<yyyy-mm-dd>-<slug>.md` stating the question, what was tried, what was learned, and whether it should graduate. Lab crates MAY depend on anything, including frameworks named in CON-20, and MAY break, be deleted, or contradict a spec. Nothing in `lab/` MAY be cited as a result.

**CON-24** Graduation: an idea moves from `lab/` to the substrate when someone wants to cite a number from it. Graduation means, in this order: a `spec-change` PR adding the spec and IDs; a candidate hypothesis file promoted to `hypotheses/`; tests citing the IDs; the control (CON-18). New POCs are expected to arrive this way; the catalogue in the ACN report is the current map, not a closed list, and `TASKS.md` carries an open exploration slot in every milestone.

## 8. Glossary

- **Bundle** — the directory produced by one run: manifest (JSON), traces (Parquet), verdict (JSON), logs.
- **Scenario** — a TOML file naming link models, schedules and (optionally) a measured trace to replay.
- **`sim` / `live` / `netem`** — execution modes of `acn-emu`; see PLAN.md §2.
- **Control** — the non-agent or plain-RPC workload run under the same scenario as the treatment.
- **Falsifier** — a predicate over bundle statistics that, if true, refutes the hypothesis.
- **Candidate / frozen** — hypothesis lifecycle states (CON-17). Candidates are for exploring; frozen ones are for citing.
- **Lab note** — the only required output of an exploration (CON-23).
