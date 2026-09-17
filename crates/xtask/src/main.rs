//! `cargo xtask <task>` — CON-8: exactly one JSON object on stdout, logs on
//! stderr, exit 0 iff `"ok": true`.
#![forbid(unsafe_code)]

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use xtask::{docs_inventory, env_hash, logging, trace_check, workspace};

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
    }
}

fn main() {
    logging::init();
    let out = run();
    println!("{out}");
    let ok = out.get("ok").and_then(Value::as_bool) == Some(true);
    std::process::exit(if ok { 0 } else { 1 });
}
