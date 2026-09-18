//! Static checks on the workspace itself: layout (CON-6), toolchain and
//! edition (CON-2), lint posture (CON-19, CON-5), the gate chain (CON-9) and
//! dependency independence (CON-20).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // CON-19: tests are exempt

mod common;

use std::fs;

use common::repo_root;

const CRATES: &[&str] = &[
    "acn-trace",
    "acn-emu",
    "acn-mockllm",
    "acn-harness",
    "acn-gen",
    "acn-replay",
    "acn-ctl",
    "acn-hyp",
    "acn-attrib",
    "acn-cli",
    "xtask",
];

const DIRS: &[&str] = &[
    "specs",
    "hypotheses",
    "scenarios/synthetic",
    "scenarios/measured",
    "tests/accept",
    "tools",
    "docs/decisions",
    "docs/gates",
    "docs/generated",
    "docs/lab",
];

fn read(rel: &str) -> String {
    let p = repo_root().join(rel);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn toml_of(rel: &str) -> toml::Value {
    toml::from_str(&read(rel)).unwrap_or_else(|e| panic!("parse {rel}: {e}"))
}

fn crate_manifests() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = CRATES
        .iter()
        .map(|c| (format!("crates/{c}"), format!("crates/{c}/Cargo.toml")))
        .collect();
    v.push((
        "tests/accept".to_owned(),
        "tests/accept/Cargo.toml".to_owned(),
    ));
    v
}

/// Cites: CON-6
#[test]
fn layout_matches_the_constitution() {
    let root = repo_root();
    for d in DIRS {
        assert!(root.join(d).is_dir(), "missing directory {d}");
    }
    for c in CRATES {
        assert!(
            root.join("crates").join(c).join("Cargo.toml").is_file(),
            "missing crate {c}"
        );
    }
    let gitignore = read(".gitignore");
    assert!(
        gitignore
            .lines()
            .any(|l| l.trim() == "/runs/" || l.trim() == "runs/"),
        "runs/ must be gitignored"
    );
}

/// Cites: CON-6
#[test]
fn workspace_members_are_exactly_the_layout() {
    let ws = toml_of("Cargo.toml");
    let mut members: Vec<String> = ws["workspace"]["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|m| m.as_str().expect("str").to_owned())
        .collect();
    members.sort();
    let mut expected: Vec<String> = CRATES.iter().map(|c| format!("crates/{c}")).collect();
    expected.push("tests/accept".to_owned());
    expected.sort();
    assert_eq!(members, expected);
    let accept = toml_of("tests/accept/Cargo.toml");
    assert_eq!(accept["package"]["name"].as_str(), Some("acn-accept"));
}

/// Cites: CON-2
#[test]
fn toolchain_is_pinned_stable_and_edition_2024() {
    let tc = toml_of("rust-toolchain.toml");
    let channel = tc["toolchain"]["channel"].as_str().expect("channel");
    let parts: Vec<&str> = channel.split('.').collect();
    assert_eq!(
        parts.len(),
        3,
        "channel must be a pinned X.Y.Z, got {channel}"
    );
    assert!(
        parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())),
        "{channel}"
    );

    let ws = toml_of("Cargo.toml");
    assert_eq!(ws["workspace"]["package"]["edition"].as_str(), Some("2024"));
    let msrv = ws["workspace"]["package"]["rust-version"]
        .as_str()
        .expect("rust-version");
    assert!(
        channel.starts_with(msrv),
        "MSRV {msrv} must equal the pinned channel {channel}"
    );
    for (_, manifest) in crate_manifests() {
        let m = toml_of(&manifest);
        assert_eq!(
            m["package"]["edition"]["workspace"].as_bool(),
            Some(true),
            "{manifest}: edition must inherit from the workspace"
        );
    }
}

/// Cites: CON-19
#[test]
fn every_crate_forbids_unsafe_and_inherits_the_lint_posture() {
    let ws = toml_of("Cargo.toml");
    let lints = &ws["workspace"]["lints"];
    assert_eq!(lints["rust"]["unsafe_code"].as_str(), Some("forbid"));
    for lint in ["unwrap_used", "expect_used", "panic"] {
        assert_eq!(
            lints["clippy"][lint].as_str(),
            Some("deny"),
            "clippy::{lint} must be denied"
        );
    }
    for (dir, manifest) in crate_manifests() {
        let m = toml_of(&manifest);
        assert_eq!(
            m["lints"]["workspace"].as_bool(),
            Some(true),
            "{manifest}: [lints] workspace = true"
        );
        let root = repo_root().join(&dir);
        let mut roots = vec![root.join("src/lib.rs"), root.join("src/main.rs")];
        roots.retain(|p| p.is_file());
        assert!(!roots.is_empty(), "{dir}: no src/lib.rs or src/main.rs");
        for r in roots {
            let src = fs::read_to_string(&r).expect("read");
            assert!(
                src.contains("#![forbid(unsafe_code)]"),
                "{} must carry #![forbid(unsafe_code)]",
                r.display()
            );
        }
    }
}

/// Cites: CON-5
#[test]
fn clippy_disallows_ambient_time_and_randomness() {
    // Clause (b) of CON-5: the lint configuration that bans ambient clocks and RNGs.
    let cfg = toml_of("clippy.toml");
    let paths: Vec<&str> = cfg["disallowed-methods"]
        .as_array()
        .expect("disallowed-methods")
        .iter()
        .map(|e| e["path"].as_str().expect("path"))
        .collect();
    for p in [
        "std::time::Instant::now",
        "std::time::SystemTime::now",
        "tokio::time::Instant::now",
        "rand::thread_rng",
        "std::thread::sleep",
    ] {
        assert!(paths.contains(&p), "clippy.toml must disallow {p}");
    }
    let ws = toml_of("Cargo.toml");
    assert_eq!(
        ws["workspace"]["lints"]["clippy"]["disallowed_methods"].as_str(),
        Some("deny")
    );
}

/// Cites: CON-9
#[test]
fn ci_script_chains_the_gates_in_order() {
    let script = read("tools/ci.sh");
    let joined: String = script
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .map(|l| l.trim_end_matches('\\').trim())
        .collect::<Vec<_>>()
        .join(" ");
    let steps: Vec<&str> = joined
        .split("&&")
        .filter_map(|s| s.find("cargo").map(|i| s[i..].trim()))
        .collect();
    assert_eq!(
        steps,
        vec![
            "cargo fmt --all --check",
            "cargo clippy --workspace --all-targets --all-features -- -D warnings",
            "cargo test --workspace",
            "cargo xtask trace-check",
            "cargo xtask docs-inventory --check",
            "cargo xtask env-hash --check",
            "cargo deny check",
        ]
    );
    assert!(script.contains("set -euo pipefail"));
    let alias = read(".cargo/config.toml");
    assert!(
        alias.contains("xtask"),
        ".cargo/config.toml must alias `cargo xtask`"
    );
}

/// Cites: CON-20
#[test]
fn no_substrate_crate_requires_a_robotics_or_dataflow_framework() {
    let banned = ["dora", "ros", "rclrs", "r2r", "aisle", "genesis"];
    for (_, manifest) in crate_manifests() {
        let m = toml_of(&manifest);
        for table in ["dependencies", "dev-dependencies", "build-dependencies"] {
            let Some(deps) = m.get(table).and_then(toml::Value::as_table) else {
                continue;
            };
            for name in deps.keys() {
                let lower = name.to_ascii_lowercase();
                assert!(
                    !banned.iter().any(|b| lower == *b
                        || lower.starts_with(&format!("{b}-"))
                        || lower.starts_with(&format!("{b}_"))),
                    "{manifest}: `{name}` is a banned required dependency (CON-20)"
                );
            }
        }
    }
}

/// Cites: CON-23
#[test]
fn lab_is_outside_the_workspace_and_outside_the_determinism_lints() {
    let ws = toml_of("Cargo.toml");
    let exclude: Vec<&str> = ws["workspace"]["exclude"]
        .as_array()
        .expect("workspace.exclude")
        .iter()
        .map(|e| e.as_str().expect("str"))
        .collect();
    assert!(
        exclude.contains(&"lab"),
        "lab/ must be excluded or cargo rejects every lab crate"
    );
    // Clippy uses the nearest clippy.toml walking up from the crate, so lab/ needs
    // its own or the CON-5 bans (Instant::now, …) would fail the lab gate.
    let lab_cfg = toml_of("lab/clippy.toml");
    assert!(
        lab_cfg.get("disallowed-methods").is_none() && lab_cfg.get("disallowed-types").is_none(),
        "lab/clippy.toml must not carry the substrate bans"
    );
    let gitignore = read(".gitignore");
    assert!(
        gitignore.lines().any(|l| l.trim() == "target/"),
        "lab crates build into their own target/; ignore it at any depth"
    );
}

/// Cites: CON-5
#[test]
fn clippy_bans_every_ambient_entropy_and_clock_source_and_unordered_maps() {
    let cfg = toml_of("clippy.toml");
    let methods: Vec<&str> = cfg["disallowed-methods"]
        .as_array()
        .expect("disallowed-methods")
        .iter()
        .map(|e| e["path"].as_str().expect("path"))
        .collect();
    for p in [
        "rand::rng",
        "rand::random",
        "rand::thread_rng",
        "fastrand::Rng::new",
        "getrandom::fill",
        "uuid::Uuid::new_v4",
        "chrono::Utc::now",
        "chrono::Local::now",
        "time::OffsetDateTime::now_utc",
        "std::collections::hash_map::RandomState::new",
    ] {
        assert!(methods.contains(&p), "clippy.toml must disallow {p}");
    }
    let types: Vec<&str> = cfg["disallowed-types"]
        .as_array()
        .expect("disallowed-types")
        .iter()
        .map(|e| e["path"].as_str().expect("path"))
        .collect();
    for t in ["std::collections::HashMap", "std::collections::HashSet"] {
        assert!(types.contains(&t), "clippy.toml must disallow {t}");
    }
    let ws = toml_of("Cargo.toml");
    assert_eq!(
        ws["workspace"]["lints"]["clippy"]["disallowed_types"].as_str(),
        Some("deny")
    );
}

/// Cites: CON-20
#[test]
fn cargo_deny_bans_the_frameworks_transitively_and_unknown_sources() {
    let deny = toml_of("deny.toml");
    let banned: Vec<&str> = deny["bans"]["deny"]
        .as_array()
        .expect("bans.deny")
        .iter()
        .map(|e| e["crate"].as_str().expect("crate"))
        .collect();
    for c in ["dora-node-api", "dora-core", "rclrs", "r2r", "rosrust"] {
        assert!(banned.contains(&c), "deny.toml must ban {c} (CON-20)");
    }
    assert_eq!(deny["sources"]["unknown-registry"].as_str(), Some("deny"));
    assert_eq!(deny["sources"]["unknown-git"].as_str(), Some("deny"));
}

/// Cites: CON-1
#[test]
fn ci_builds_on_macos_arm64_and_linux_x86_64_and_aarch64() {
    let ci = read(".github/workflows/ci.yml");
    for runner in ["macos-latest", "ubuntu-latest", "ubuntu-24.04-arm"] {
        assert!(ci.contains(runner), "CI matrix must include {runner}");
    }
    assert!(
        !ci.contains("rust-toolchain@stable"),
        "CI must use the pinned toolchain from rust-toolchain.toml (CON-2), not floating stable"
    );
}

/// Cites: CON-14, CON-7
#[test]
fn ci_runs_pr_check_with_the_labels_and_reruns_when_labels_change() {
    let ci = read(".github/workflows/ci.yml");
    assert!(ci.contains("cargo xtask pr-check"), "CI must run pr-check");
    for ty in ["labeled", "unlabeled"] {
        assert!(ci.contains(ty), "pull_request types must include `{ty}`");
    }
    assert!(
        ci.contains("fetch-depth: 0"),
        "pr-check diffs against the base branch and needs history"
    );
}

/// Cites: CON-12
#[test]
fn ci_has_a_nightly_trigger_for_the_nightly_tiers() {
    let ci = read(".github/workflows/ci.yml");
    assert!(
        ci.contains("schedule:") && ci.contains("cron:"),
        "nightly tiers need a schedule trigger"
    );
}
