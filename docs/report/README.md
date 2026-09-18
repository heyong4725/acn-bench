# The source report

acn-bench implements the experimental programme of the ACN technical report, *Agentic Communication Networks — A Technical Analysis of What Changes When the Endpoint Is a Loop*.

| | |
|---|---|
| Version the specs target | v1.2 (September 2026) |
| SHA-256 of that version | `2fa7cb1b59e0223cf9f1eb19a11e8bd73ab1edb37406fde77bc0d54188d401f6` |
| Vendored here | no — publishing the report is the owner's decision |

The hash is a commitment: anyone holding the report can check that they have the version the specs were written against.

## Where the repo reads from it

| Report | Repo |
|---|---|
| Appendix E design rule (hypothesis, variables, measures, control, falsifier; files no loop can edit) | `specs/000-constitution.md` CON-17, CON-18; `hypotheses/` |
| Appendix E shared substrate (emulation, workload generator, replayer, control and evidence plane) | `crates/acn-emu`, `acn-gen`, `acn-replay`, `acn-ctl`; PLAN.md §3 |
| Appendix E catalogue, POC 1a–16 | one spec in the 100-series and one hypothesis file per POC; TASKS.md |
| Appendix E on where an auto-research loop adds value | `specs/085-feedback-loop.md` |
| Traffic model and parameter sheet (§3.4, Appendix C) | `acn-gen` (SPEC 050, to write) |
| Measurement methodology (§3.5) | `specs/010-trace-schema.md` derived views |
| What the harness controls (§3.6) | POC 4, `acn-harness` (SPEC 040, to write) |

If the report is revised, update the version and hash here in the same PR that adapts the specs.
