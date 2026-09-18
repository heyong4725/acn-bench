# The source report

acn-bench implements the experimental programme of:

> **Agentic Communication Networks — A Technical Analysis of What Changes When the Endpoint Is a Loop**
> Young He, with Claude. Version 1.2, September 2026. Prepared for Futurewei technology strategy; input to the IEEE ComSoc ACN Technical Working Group.

| | |
|---|---|
| Version the specs target | v1.2 |
| File | `ACN-Agentic-Communication-Networks-Technical-Report_1.md` |
| SHA-256 | `2fa7cb1b59e0223cf9f1eb19a11e8bd73ab1edb37406fde77bc0d54188d401f6` |
| Vendored here | no — the report is a working document; publication is the owner's decision |

## Where the repo reads from it

| Report | Repo |
|---|---|
| Appendix E §E.1 design rule (hypothesis → varies → measures → control → falsifier; files no loop can edit) | `specs/000-constitution.md` CON-17, CON-18; `hypotheses/` |
| Appendix E §E.2 shared substrate (emulation, workload generator, replayer, control and evidence plane) | `crates/acn-emu`, `acn-gen`, `acn-replay`, `acn-ctl`; PLAN.md §3 |
| Appendix E §E.3 catalogue, POC 1a–16 | one spec in the 100-series and one hypothesis file per POC; TASKS.md |
| Appendix E §E.4 where an auto-research loop adds value | `specs/085-feedback-loop.md` |
| §3.4 traffic model, Appendix C parameter sheet | `acn-gen` (SPEC 050, to write) |
| §3.5 measurement methodology | `specs/010-trace-schema.md` derived views |
| §3.6 what the harness controls | POC 4, `acn-harness` (SPEC 040, to write) |
| Revision ledger `H-n`, §14.9 gap table `G-n` | `report_refs` in hypothesis files |

If the report is revised, update the version and hash here in the same PR that adapts the specs.
