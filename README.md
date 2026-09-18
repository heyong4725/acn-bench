# acn-bench

[![ci](https://github.com/heyong4725/acn-bench/actions/workflows/ci.yml/badge.svg)](https://github.com/heyong4725/acn-bench/actions/workflows/ci.yml)

A Rust workspace that builds the experimental substrate for **Agentic Communication Networks (ACN)** and runs its proof-of-concept catalogue as frozen, falsifiable hypotheses with mandatory controls. It implements the programme in Appendix E of the ACN technical report ([what the repo reads from it](docs/report/README.md)).

The question it exists to answer: *when the endpoint is an agent loop rather than a person, where does the network actually matter, and by how much?* Every experiment is stated so that "it does not" is a publishable result.

**Status:** bootstrap (milestone M0 in progress). The workspace, gates and tooling exist; the trace schema (T02) is next. See [`TASKS.md`](TASKS.md).

## How it is organised

| Track | Where | Rules |
|---|---|---|
| Exploration | [`lab/`](lab/README.md) | Any Rust, any dependency. `fmt`, `clippy`, and a lab note. Nothing here is citable. |
| Substrate | `crates/`, `specs/`, `hypotheses/` | Specs with requirement IDs, tests that cite them, deterministic runs, a control for every experiment, a frozen set that only a human-merged `env-change` PR can alter. |

An idea graduates from lab to substrate when someone wants to cite a number from it.

```
specs/        numbered specs; every requirement has an ID (CON-5, TRC-24, …)
hypotheses/   one TOML per POC: hypothesis, variables, control, falsifier   [frozen]
scenarios/    synthetic link scenarios; measured impairment traces          [measured: frozen]
crates/       acn-trace · acn-emu · acn-mockllm · acn-harness · acn-gen · acn-replay
              acn-ctl · acn-hyp · acn-attrib · acn-cli · xtask
tests/accept/ one acceptance suite per POC
lab/          spikes and candidate hypotheses
docs/         decisions (ADRs), gates, generated inventory, lab notes
```

Start with [`PLAN.md`](PLAN.md) for the design, [`specs/000-constitution.md`](specs/000-constitution.md) for the invariants, and [`GETTING-STARTED.md`](GETTING-STARTED.md) to set up a machine.

## Build and check

```bash
rustup show          # installs the toolchain pinned in rust-toolchain.toml
tools/ci.sh          # fmt, clippy -D warnings, tests, trace-check, docs-inventory, env-hash, cargo-deny
```

`cargo xtask trace-check` fails when an implemented requirement has no citing test, when a test cites an ID that does not exist, or when a document refers to one. The current state of every requirement is in [`docs/generated/requirements.md`](docs/generated/requirements.md).

## Principles worth knowing before you read the code

- **Determinism is a feature.** Clocks and randomness are injected; in `sim` mode the same inputs produce a byte-identical bundle.
- **Sim has a live twin.** A simulated number is cited only alongside a live run of the same scenario and the recorded divergence.
- **Mock is not a result.** Bundles produced on the mock inference server gate the suite; they are never cited.
- **Negative results are reported, not re-hypothesised.** Hypothesis files are frozen before the run that tests them.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). Agents follow [`CLAUDE.md`](CLAUDE.md). Licence: [Apache-2.0](LICENSE).
