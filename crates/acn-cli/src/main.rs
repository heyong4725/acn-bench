//! `acn` — the acn-bench command line (CON-8: one JSON object on stdout,
//! logs on stderr, exit 0 iff `"ok": true`). Subcommands land with their specs;
//! T01 ships `version` only.
#![forbid(unsafe_code)]

use clap::{CommandFactory, Parser, Subcommand};
use serde_json::{Value, json};
use std::io::IsTerminal as _;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "acn",
    version,
    about = "ACN experimental substrate",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print the version of the `acn` binary.
    Version,
}

fn init_logging() {
    let spec = std::env::var("ACN_LOG").unwrap_or_else(|_| "info".to_owned());
    let builder = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal());
    if spec == "json" {
        builder
            .json()
            .with_env_filter(EnvFilter::new("info"))
            .init();
    } else {
        builder.with_env_filter(EnvFilter::new(spec)).init();
    }
}

fn run() -> Value {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            use clap::error::ErrorKind::{DisplayHelp, DisplayVersion};
            if matches!(e.kind(), DisplayHelp | DisplayVersion) {
                eprint!("{e}");
                return json!({ "ok": true, "command": Cli::command().get_name() });
            }
            return json!({ "ok": false, "error": e.to_string() });
        }
    };
    match cli.cmd {
        Cmd::Version => json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") }),
    }
}

fn main() {
    init_logging();
    let out = run();
    println!("{out}");
    let ok = out.get("ok").and_then(Value::as_bool) == Some(true);
    std::process::exit(if ok { 0 } else { 1 });
}
