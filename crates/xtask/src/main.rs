//! `cargo xtask <task>` — CON-8: exactly one JSON object on stdout, logs on
//! stderr, exit 0 iff `"ok": true`.
#![forbid(unsafe_code)]

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use xtask::{docs_inventory, env_hash, logging, pr_check, trace_check, workspace};

#[derive(Parser)]
#[command(
    name = "xtask",
    about = "acn-bench repository tasks",
    disable_help_subcommand = true
)]
struct Cli {
    /// Workspace root (default: the repository containing this crate).
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// CON-12: every implemented MUST is cited by a test; every citation names a real ID.
    TraceCheck,
    /// Regenerate docs/generated/ (or, with --check, fail if it is out of date).
    DocsInventory {
        #[arg(long)]
        check: bool,
    },
    /// CON-7: hash the frozen set; --check against env-hash.json, --write to update it.
    EnvHash {
        #[arg(long, conflicts_with = "write")]
        check: bool,
        #[arg(long)]
        write: bool,
    },
    /// CON-14, CON-7, LOOP-20: PR label rules and CODEOWNERS coverage of protected paths.
    PrCheck {
        /// Diff against this git ref (`git diff --name-only <base>...HEAD`).
        #[arg(long, conflicts_with = "changed")]
        base: Option<String>,
        /// Comma-separated changed paths, instead of reading them from git.
        #[arg(long)]
        changed: Option<String>,
        /// Comma-separated PR labels.
        #[arg(long, default_value = "")]
        labels: String,
    },
}

fn to_json<T: serde::Serialize>(r: xtask::Result<T>) -> Value {
    match r.and_then(|v| serde_json::to_value(v).map_err(Into::into)) {
        Ok(v) => v,
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

fn run() -> Value {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            use clap::error::ErrorKind::{DisplayHelp, DisplayVersion};
            if matches!(e.kind(), DisplayHelp | DisplayVersion) {
                eprint!("{e}");
                return json!({ "ok": true });
            }
            return json!({ "ok": false, "error": e.to_string() });
        }
    };
    let root = cli.root.unwrap_or_else(workspace::default_root);
    match cli.cmd {
        Cmd::TraceCheck => to_json(trace_check::run(&root)),
        Cmd::DocsInventory { check } => to_json(docs_inventory::run(&root, check)),
        Cmd::EnvHash { check, write } => {
            let mode = if check {
                env_hash::Mode::Check
            } else if write {
                env_hash::Mode::Write
            } else {
                env_hash::Mode::Print
            };
            to_json(env_hash::run(&root, mode))
        }
        Cmd::PrCheck {
            base,
            changed,
            labels,
        } => {
            let split = |s: &str| -> Vec<String> {
                s.split(',')
                    .map(str::trim)
                    .filter(|x| !x.is_empty())
                    .map(str::to_owned)
                    .collect()
            };
            let changes = match (base, changed) {
                (Some(b), _) => pr_check::Changes::GitBase(b),
                (None, Some(c)) => pr_check::Changes::List(split(&c)),
                (None, None) => pr_check::Changes::None,
            };
            to_json(pr_check::run(&root, changes, &split(&labels)))
        }
    }
}

fn main() {
    logging::init();
    let out = run();
    let ok = out.get("ok").and_then(Value::as_bool) == Some(true);
    // CON-8: the JSON object is the result. If it cannot be written (a closed pipe),
    // the run has not succeeded, and `println!` would panic instead of saying so.
    let written = {
        use std::io::Write as _;
        let mut stdout = std::io::stdout().lock();
        writeln!(stdout, "{out}")
            .and_then(|()| stdout.flush())
            .is_ok()
    };
    std::process::exit(if ok && written { 0 } else { 1 });
}
