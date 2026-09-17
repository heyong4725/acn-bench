# Getting started on a Mac

Everything below is copy-paste. Steps 1–4 are one-time setup; 5–7 are the first working day.

## 1. Toolchain (once)

```bash
xcode-select --install                       # compilers, git
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # rustup; then restart the shell
rustup show                                   # picks up rust-toolchain.toml automatically inside the repo
cargo install cargo-deny cargo-nextest --locked
brew install gh                               # GitHub CLI
gh auth login
```

## 2. Create the public repo

```bash
mkdir -p ~/src && cd ~/src
unzip ~/Downloads/acn-bench-plan.zip && mv acnbench acn-bench && cd acn-bench
git init -b main
gh repo create acn-bench --public --license apache-2.0 --source=. --remote=origin --description "ACN experimental substrate and POC catalogue (Rust)"
git add -A && git commit -m "chore: plan, constitution, SPEC 010, tasks, workspace skeleton"
git push -u origin main
```

`gh repo create --license apache-2.0` adds the LICENSE file; add a NOTICE line with the Futurewei copyright if legal asks for one.

Then, in the GitHub UI or with `gh`:

```bash
gh label create spec-change   --color 1d76db --description "Edits under specs/ (CON-14)"
gh label create env-change    --color d93f0b --description "Edits to the frozen set (CON-7)"
gh label create spec-conflict --color b60205 --description "A spec and a test disagree (CON-13)"
gh label create lab           --color 0e8a16 --description "Exploration track (CON-23)"
gh api -X PUT repos/{owner}/acn-bench/branches/main/protection \
  -f required_status_checks.strict=true -f 'required_status_checks.contexts[]=gates (macos-latest)' \
  -f 'required_status_checks.contexts[]=gates (ubuntu-latest)' -f enforce_admins=false \
  -f required_pull_request_reviews.required_approving_review_count=1 -F restrictions=null
```

Squash-merge only (repo Settings → General → Pull Requests): the PR title becomes the mainline commit subject (CON-11).

## 3. Secrets

```bash
cp .env.example .env            # fill in ANTHROPIC_API_KEY and OPENAI_API_KEY; .env is gitignored
```

Both providers are needed by T06 (per-provider POC 4, CON-26). Nothing before T06 reads them.

## 4. Expect the workspace not to build yet

`Cargo.toml` lists eleven crates that do not exist until T01 scaffolds them; `cargo build` fails until then. That is the first task's job, not a bug. Dependency versions in `Cargo.toml` are placeholders: T01 runs `cargo update`, fixes any version that does not resolve, and commits `Cargo.lock`.

## 5. Day one: two Claude Code sessions in parallel

Open two terminals in the repo. One drives the substrate, one drives the lab.

**Terminal A — substrate, T01.** Start `claude` and paste the T01 kickoff prompt from `TASKS.md`:

```
Read CLAUDE.md and specs/000-constitution.md. Implement task T01 from TASKS.md:
bootstrap the workspace exactly per CON-6, CON-9, CON-12, CON-19, tests first
(crates/xtask/tests/trace_check_selfhost.rs, crates/xtask/tests/env_hash.rs).
Do not add acn-emu, tokio proxies or any network code yet. Finish with all gates
green and a PR description listing requirement IDs.
```

When it opens the PR, start a *second* `claude` session (or a different agent) for the cross-review prompt in `TASKS.md` (CON-16). Merge yourself after review.

**Terminal B — lab, spike (a).** `git switch -c lab/turn-transport`, start `claude`, paste the lab-spike prompt from `TASKS.md` with:

```
Question: how much of POC 11/12/14 does a turn-native transport collapse into one
mechanism? Build lab/turn-transport: a quinn (QUIC) client/server where one stream
carries one agent turn with a declared deadline, the stream resumes after a simulated
link gap without re-sending the prefix, and the prefix is sent as a delta against what
the server already holds. Measure bytes on the wire and turn completion time with and
without a 2-second gap, against a plain HTTP/1.1 + SSE baseline in the same crate.
```

It needs no substrate; it needs only Rust. The lab note is the deliverable.

## 6. Day two onward

Substrate: T02 (SPEC 010 is already drafted — the agent implements it), then T03, T04, T05, T05b (the loop runner, SPEC 085 — also drafted), T06 in order, one PR each, cross-reviewed. Specs 030/040/080/095/100 are *not* drafted yet: each of those tasks starts with the spec-drafting prompt from `TASKS.md` as a `spec-change` PR you merge, then the implementation PR. Lab: spike (b) `lab/trace-capture` needs a phone tethered on 5G and any server you can reach (a $5 VPS is enough); do the walk once, land the trace via T08. Spike (c) `lab/hypotheses/p17-a2a.toml` is already written; iterate on it when the generator exists.

## 6a. Empty directories

Git does not track empty directories. `docs/gates`, `docs/generated`, `docs/lab`, `scenarios/*`, `tests/accept`, `crates` and `lab` ship empty; T01 creates the crates and adds `.gitkeep` files where a directory must exist before it has content.

## 7. Weekly

`cargo xtask docs-inventory` regenerates `docs/generated/`; review lab notes in `docs/lab/` and decide graduate / park / drop; anything graduating starts with a `spec-change` PR.

## What is deliberately not on the Mac

Linux `netem` (M4) — a Linux box or the ubuntu CI runner. A GPU inference node for POC 7 (M3) — external, reached through `acn-ctl`. Neither blocks M0–M2.
