# SPEC 080 — Hypothesis files, falsifier predicates and verdicts

**Status:** Draft v0.1 (September 2026). **Inherits:** SPEC 000, 010. **Prefix:** HYP. **Crate:** `acn-hyp` (frozen set, CON-7).
**Purpose:** define the file in which a POC states its hypothesis before it runs, the small typed language in which its falsifier is written, and the only path from a bundle to a verdict — such that a negative result cannot be edited into a positive one, by a person or by a loop, without changing a hash that every bundle carries.

## 0. Why a grammar

The report's design rule (Appendix E §E.1) is that hypotheses and falsifiers are fixed before the experiment runs, in a file no automated search loop can edit. That rule has teeth only if the falsifier is *evaluated by a machine*: a falsifier written as free text is re-interpreted every time a result disappoints. This spec therefore fixes a file format that parses strictly (HYP-1..9), a predicate language small enough to specify completely (HYP-10..16), and verdict semantics with no discretionary step (HYP-20..27).

## 1. Definitions

- **Hypothesis file** — one TOML file per POC, `<poc>.toml`. **Frozen** when it lives under `hypotheses/`; **candidate** when it lives under `lab/hypotheses/` (CON-17).
- **Arm** — `treatment` or `control`; a run belongs to exactly one (`acn.role`, TRC-10).
- **Cell** — one assignment of values to every `[varies]` parameter. **Replicate** — one seeded run of one arm in one cell.
- **Quantity** — a named statistic of a view column (SPEC 010 §7) over the runs of one arm in one cell, e.g. `cost_per_success`, `ttft_p99_ms`.
- **Verdict** — `pass`, `fail` or `inconclusive`, with labels.

## 2. File format

**HYP-1** A hypothesis file MUST be TOML and MUST deserialise into the typed structure of this section with unknown keys rejected at every level (`deny_unknown_fields`). A file that does not parse MUST make every command that reads it fail with the key path and the reason; it MUST NOT be partially honoured.

**HYP-2** The file MUST contain the tables `[poc]`, `[hypothesis]`, `[varies]`, `[measures]`, `[control]`, `[design]`, `[falsifier]` and `[expected]`. `[poc]` MUST carry `id` (matching the file stem) and `title`, and MAY carry `spec` (path of the POC spec; MUST exist, or be listed in `specs/README.md`, for a frozen file), `report_refs` (report sections, `H-n` and `G-n` identifiers) and `status`.

**HYP-3** Status is decided by location: a file under `hypotheses/` is `frozen`, a file under `lab/hypotheses/` is `candidate`. If `[poc].status` is present it MUST equal the status its location implies; a mismatch is a parse error. Every bundle MUST record the status so determined (`acn.hypothesis.status`, TRC-10).

**HYP-4** No automated agent, search loop or CI job MAY hold write access to `hypotheses/`. `acn-hyp` and everything that links it MUST open hypothesis files read-only, MUST NOT write anywhere outside `runs/`, and MUST abort with `hypothesis_changed` if the hash of a hypothesis file changes while a loop is using it (LOOP-13).

**HYP-5** `hypothesis_hash` is `blake3` of the file's bytes as stored (CON-27a). Whitespace and comments are therefore part of the hash: a frozen file is not reformatted.

**HYP-6** `[varies]` maps each parameter name to exactly one of `{ kind = "bool" }`, `{ kind = "enum", values = [...] }`, `{ kind = "range", min = x, max = y }` (numeric, inclusive) or `{ kind = "int_range", min = a, max = b }`. These are the only parameters a loop may vary (LOOP-10), and a sample outside the declared domain MUST be rejected before the run starts. A parameter MAY carry `pooled = false`, in which case results are reported per value and never pooled across values (required for `provider`, CON-26).

**HYP-7** `[measures]` MUST carry `primary` and MAY carry `secondary`, each a list of quantity names. Every name MUST resolve to a quantity definition (HYP-12) at parse time. A predicate MAY reference only quantities listed here.

**HYP-8** `[control]` MUST carry a `description` and, where the control is a configuration of the same system, a `config` table assigning a value to parameters of `[varies]`; each assigned value MUST lie in the parameter's domain. Where the control is a different workload (plain RPC, file transfer), `[control].workload` MUST name the generator mode that produces it (SPEC 050). A file without a control MUST NOT parse (CON-18).

**HYP-9** `[design]` MUST carry `search` (`grid`|`bisect`|`random`), `replicates` (integer ≥ 2) and `twin_required` (bool; `false` is permitted only for network-free hypotheses, LOOP-4), and MAY carry `seeds` (`"derived"`, meaning CON-27d), `backends`, `min_providers_for_verdict` and `sim_live_tolerance` (a table of quantity name → relative tolerance, CON-25). `[expected]` MUST carry `outcome` (`pass`|`fail`) and MAY carry `note`; it is recorded in the verdict and has no effect on it.

## 3. The predicate language

**HYP-10** `[falsifier].predicate` and the optional `[falsifier].inconclusive_if` are strings in the language below, parsed when the file is loaded. A predicate that does not parse or does not type-check MUST make the file fail to load. The falsifier evaluates to a boolean: **true means the hypothesis is refuted**.

```
expr    := or
or      := and ( "or" and )*
and     := not ( "and" not )*
not     := "not" not | quant
quant   := cmp ( "at" ( "all" | "any" ) ident cmpop number )?
cmp     := sum ( cmpop sum )?
cmpop   := "<" | "<=" | ">" | ">=" | "==" | "!="
sum     := term ( ( "+" | "-" ) term )*
term    := unary ( ( "*" | "/" ) unary )*
unary   := "-" unary | atom
atom    := number | call | ident | "(" expr ")"
call    := ident "(" ( arg ( "," arg )* )? ")"
arg     := ident "=" number | expr
ident   := [a-z_][a-z0-9_]*
number  := decimal literal, optional fraction and exponent
```

**HYP-11** The language has two types, *number* and *boolean*, and no implicit conversion. Comparison takes numbers and yields a boolean; `and`, `or`, `not` take booleans. A comparison involving a value that is undefined (no runs, division by zero, a quantity with no data) is *undefined*, and an undefined falsifier yields the verdict `inconclusive` with the reason recorded, never `pass`.

**HYP-12** Identifiers resolve, in this order, to: the run-set counters `replicates` (minimum completed replicates over the arms and cells evaluated) and `providers_reported`; a parameter of `[varies]` (only inside an `at` clause); a quantity of `[measures]`. A bare quantity name means its value in the treatment arm. `q(control)`, `q(treatment)` and `q(v)` for an enum value `v` of a non-pooled parameter select the arm or the value. Quantity definitions — name, view, column, statistic (`mean`, `p50`, `p99`, `ratio`, `count`) and unit — MUST live in one table in `acn-hyp`, MUST read only views and `acn.*` promoted columns (TRC-3), and the table MUST be rendered by `cargo xtask docs-inventory`.

**HYP-13** Built-in functions, all over numbers: `abs(x)`; `min(x, y)`, `max(x, y)`; `effect(q)` = `q(treatment) − q(control)` within a cell; `rel_effect(q)` = `effect(q) / q(control)`; `ci_low(q, ci = 0.95)` and `ci_high(q, ci = 0.95)`, the bounds of the bootstrap confidence interval of `effect(q)` (SPEC 090); `noise_floor(q, control, ci = 0.95)`, the half-width of the confidence interval of the difference between two disjoint halves of the control arm's replicates; `max_over_knobs(x)` and `min_over_knobs(x)`, the extremum of `x` over all cells. No other function exists; an unknown name is a parse error.

**HYP-14** `P at all v <= c` holds when `P` holds in every evaluated cell whose parameter `v` satisfies the bound, and `at any` when it holds in at least one; with no such cell the result is undefined (HYP-11). Without an `at` clause, a predicate that references per-cell values outside `max_over_knobs`/`min_over_knobs` MUST be rejected as ambiguous when the design has more than one cell.

**HYP-15** Evaluation MUST be deterministic: fixed cell order (lexicographic by parameter name, then value), fixed summation order, bootstrap resampling from the sub-stream `"hyp.bootstrap"` (CON-27c) with a resample count fixed in `acn-hyp`. The same bundles and the same hypothesis file MUST yield a byte-identical `verdict.json`.

**HYP-16** Both hypothesis files that exist when this spec is accepted MUST parse under it without edits to their predicates: `hypotheses/p4.toml` (`max_over_knobs(abs(effect(cost_per_success))) < noise_floor(cost_per_success, control, ci = 0.95)`, `inconclusive_if = "replicates < 20 or providers_reported < 2"`) and `lab/hypotheses/p17-a2a.toml` (a difference of `q(a2a)` and `q(control)` under `at all rtt_ms <= 150`). A file that needs an edit to conform is reported to the owner, not edited by the implementer (CON-7).

## 4. Verdicts

**HYP-20** `acn hyp verdict --hypothesis <file> <bundle>…` is the only path from bundles to a verdict (CON-17). It MUST verify each bundle (TRC-23), MUST refuse bundles whose `hypothesis.hash` differs from the file's, and MUST write `verdict.json` and print the same object (CON-8).

**HYP-21** The verdict is `inconclusive` if `inconclusive_if` is true or undefined, if the falsifier is undefined, if any arm in any evaluated cell has fewer completed replicates than `[design].replicates`, if the control arm is missing (CON-18), or if `twin_required` is true and the twin divergence is absent or out of tolerance (CON-25). Otherwise it is `fail` if the falsifier is true and `pass` if it is false. No other input affects the verdict.

**HYP-22** `verdict.json` MUST carry: `verdict`, `reasons[]`, `hypothesis {id, status, hash}`, `expected`, `labels[]`, the `run_id` of every bundle read, `env_hash`, `build_hash`, the value of every quantity and sub-expression the predicates evaluated (per cell and arm, with replicate counts and confidence intervals), and the treatment-minus-control effect of every primary quantity (CON-18).

**HYP-23** Labels: `exploratory` when the hypothesis status is `candidate` (CON-17); `mock-gated` when any bundle has `backend = "mockllm"` (CON-26); `sim-only` when no live twin was evaluated. A verdict carrying `exploratory` or `mock-gated` MUST NOT be rendered as citable on an evidence page (LOOP-30).

**HYP-24** A parameter declared `pooled = false` MUST yield one verdict per value (for `provider`, one per provider), reported side by side; `providers_reported` counts the values with a non-`inconclusive` verdict. The file-level verdict is `inconclusive` while `providers_reported < [design].min_providers_for_verdict`, `fail` if any per-value verdict is `fail`, and `pass` otherwise.

**HYP-25** A `fail` on a frozen hypothesis is a result. Tooling MUST NOT offer an option to relax, rewrite or re-scope the hypothesis, and MUST record a `fail` on the evidence page like any other verdict (LOOP-3).

**HYP-26** Freezing: a candidate becomes frozen by a Class C PR that moves the file to `hypotheses/`, removes or corrects `[poc].status`, adds its POC spec, and updates `env-hash.json`. The PR MUST precede the first run whose bundle is cited (CON-17). `cargo xtask env-hash --check` failing on a changed hypothesis file is the freeze check.

**HYP-27** `acn hyp lint <file>` MUST parse a hypothesis file, type-check its predicates, resolve its quantities and print the parsed structure, without reading any bundle; CI MUST run it on every file under `hypotheses/` and `lab/hypotheses/`.

## 5. Acceptance tests (names are normative)

- `crates/acn-hyp/tests/format.rs` — HYP-1..3, HYP-6..9: strict parsing, unknown keys, status by location, domains, mandatory control.
- `crates/acn-hyp/tests/predicate_parse.rs` — HYP-10, HYP-11, HYP-13, HYP-14: grammar, typing, unknown functions, ambiguity rejection; golden parse trees.
- `crates/acn-hyp/tests/existing_files.rs` — HYP-16: `p4.toml` and `p17-a2a.toml` parse unedited.
- `crates/acn-hyp/tests/verdict_semantics.rs` — HYP-21, HYP-23, HYP-24: each `inconclusive` cause, labels, per-provider verdicts, on synthetic bundles.
- `crates/acn-hyp/tests/verdict_determinism.rs` — HYP-15: byte-identical `verdict.json` across two evaluations.
- `crates/acn-hyp/tests/readonly.rs` — HYP-4: a hypothesis directory made writable is never written; a changed file aborts the loop.
- `tests/accept/hyp_lint.rs` — HYP-27: every hypothesis file in the repository lints.

## 6. Open questions (ADR candidates)

1. Whether `noise_floor` should be the split-half control statistic defined here or a paired A/A run; the split-half form needs no extra runs and is the default.
2. Whether `bisect` designs need a dedicated `crossing(q, threshold)` built-in so a boundary-finding falsifier (POC 1a, POC 7) can state "no crossing exists" directly.
3. Whether per-value verdicts (HYP-24) should generalise beyond `provider` to endpoint class in POC 1a.
