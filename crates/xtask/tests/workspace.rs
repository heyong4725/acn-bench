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

/// Whether a crate name belongs to a robotics or dataflow framework (CON-20).
fn is_banned_crate(name: &str) -> bool {
    let n = name.to_ascii_lowercase().replace('_', "-");
    [
        "dora",
        "aisle",
        "genesis",
        "rclrs",
        "r2r",
        "rosrust",
        "roslibrust",
    ]
    .contains(&n.as_str())
        || [
            "dora-",
            "ros2-",
            "ros2",
            "rosrust-",
            "rclrs-",
            "r2r-",
            "roslibrust-",
            "ros-",
        ]
        .iter()
        .any(|p| n.starts_with(p))
}

/// Every dependency table of a manifest: the three top-level ones and those
/// under `[target.'cfg(..)'.*]`. Yields (key, real package name).
fn dependency_names(m: &toml::Value) -> Vec<(String, String)> {
    const TABLES: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];
    let mut tables: Vec<&toml::value::Table> = TABLES
        .iter()
        .filter_map(|t| m.get(*t).and_then(toml::Value::as_table))
        .collect();
    if let Some(targets) = m.get("target").and_then(toml::Value::as_table) {
        for target in targets.values() {
            tables.extend(
                TABLES
                    .iter()
                    .filter_map(|t| target.get(*t).and_then(toml::Value::as_table)),
            );
        }
    }
    tables
        .into_iter()
        .flat_map(|t| t.iter())
        .map(|(key, spec)| {
            // `alias = { package = "real-name" }` hides the real crate behind the key.
            let package = spec
                .get("package")
                .and_then(toml::Value::as_str)
                .unwrap_or(key);
            (key.clone(), package.to_owned())
        })
        .collect()
}

/// Cites: CON-20
#[test]
fn no_substrate_crate_requires_a_robotics_or_dataflow_framework() {
    let mut manifests: Vec<String> = crate_manifests().into_iter().map(|(_, m)| m).collect();
    manifests.push("Cargo.toml".to_owned()); // [workspace.dependencies], inherited by `workspace = true`
    for manifest in manifests {
        let m = toml_of(&manifest);
        let mut names = dependency_names(&m);
        if let Some(ws) = m.get("workspace") {
            names.extend(dependency_names(ws));
        }
        for (key, package) in names {
            assert!(
                !is_banned_crate(&key) && !is_banned_crate(&package),
                "{manifest}: `{key}` (package `{package}`) is a banned required dependency (CON-20)"
            );
        }
    }
}

/// Cites: CON-20
#[test]
fn the_framework_matcher_catches_renames_and_targets_and_spares_lookalikes() {
    for banned in [
        "dora-node-api",
        "dora_core",
        "ros2-client",
        "ros2_client",
        "r2r",
        "rclrs",
        "rosrust_msg",
        "aisle",
    ] {
        assert!(is_banned_crate(banned), "{banned} must be banned");
    }
    for fine in ["rosetta", "across", "doras", "r2r2", "serde", "tokio"] {
        assert!(!is_banned_crate(fine), "{fine} must not be banned");
    }
    let m: toml::Value = toml::from_str(
        "[dependencies]\nbus = { package = \"dora-node-api\", version = \"0.3\" }\n\n[target.'cfg(unix)'.dependencies]\nros2-client = \"0.7\"\n",
    )
    .expect("toml");
    let names = dependency_names(&m);
    assert!(
        names
            .iter()
            .any(|(k, p)| k == "bus" && p == "dora-node-api"),
        "{names:?}"
    );
    assert!(names.iter().any(|(k, _)| k == "ros2-client"), "{names:?}");
}
