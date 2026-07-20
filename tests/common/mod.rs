//  Project:   macbash
//  File:      tests/common/mod.rs
//  Purpose:   Shared helpers for the end-to-end CLI integration tests
//  Language:  Rust
//
//  License:   Apache-2.0
//  Copyright: (c) 2025-2026 HYPERI PTY LIMITED

// Every integration test binary includes this module wholesale, so a helper
// used by only one of them would otherwise be reported as dead code.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

/// A script with no findings at any severity -- the "already portable" case.
pub const PORTABLE_SCRIPT: &str =
    "#!/usr/bin/env bash\nset -euo pipefail\n\nprintf '%s\\n' \"hello\"\n";

/// A custom rule pack for the `--config` path. Deliberately matches something
/// no builtin rule covers, so a hit can only have come from this file.
pub const CUSTOM_RULES_YAML: &str = concat!(
    "version: \"1.0\"\n",
    "rules:\n",
    "  - id: custom-apt-get\n",
    "    name: apt-get is Linux-only\n",
    "    description: apt-get does not exist on macOS; use brew instead.\n",
    "    severity: error\n",
    "    pattern: '\\bapt-get\\s+install\\b'\n",
    "    fix_type: suggest\n",
    "    fix_template: brew install\n",
);

/// Input for CUSTOM_RULES_YAML: line 2 is custom-only, line 3 is a builtin hit.
pub const CUSTOM_RULES_INPUT: &str =
    "#!/usr/bin/env bash\napt-get install ripgrep\necho -e \"done\"\n";

/// A fresh command bound to the freshly built `macbash` binary.
pub fn macbash() -> Command {
    Command::cargo_bin("macbash")
        .expect("macbash binary must be built before the integration tests")
}

/// Absolute path to a checked-in fixture. Fixtures are deliberately broken
/// input and are read-only -- copy one before running anything that writes.
pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Copy a fixture into `dir` under its own name and return the copy's path.
pub fn copy_fixture(dir: &TempDir, name: &str) -> PathBuf {
    let dest = dir.path().join(name);
    std::fs::copy(fixture(name), &dest).expect("copying fixture into the tempdir");
    dest
}

/// Write `body` to `dir/name` and return the path.
pub fn write_file(dir: &TempDir, name: &str, body: &str) -> PathBuf {
    let dest = dir.path().join(name);
    std::fs::write(&dest, body).expect("writing into the tempdir");
    dest
}

/// Run the binary and return its stdout as a String, ignoring the exit code.
pub fn stdout_of(cmd: &mut Command) -> String {
    let out = cmd.output().expect("running macbash");
    String::from_utf8(out.stdout).expect("stdout must be utf-8")
}

/// The JSON report for `path` at `info` severity, i.e. nothing filtered out.
pub fn json_report(path: &Path) -> String {
    stdout_of(
        macbash()
            .args(["--format", "json", "--severity", "info"])
            .arg(path),
    )
}

/// Total findings reported for `path`, at `info` severity.
pub fn finding_count(path: &Path) -> usize {
    json_uint(&json_report(path), "total_issues")
}

/// Value of a top-level unsigned field such as `total_issues`.
///
/// The dev-dependency set carries no JSON parser, so the JSON assertions scan
/// the rendered text. That is safe rather than fragile: JSON escapes any quote
/// inside a string value, so an unescaped `"<field>": ` byte sequence can only
/// ever be a real key, never part of a description or a matched source line.
pub fn json_uint(json: &str, field: &str) -> usize {
    let key = format!("\"{field}\": ");
    let at = json
        .find(&key)
        .unwrap_or_else(|| panic!("field {field:?} missing from JSON report:\n{json}"));
    let rest = &json[at + key.len()..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end]
        .parse()
        .unwrap_or_else(|_| panic!("field {field:?} is not an unsigned integer"))
}

/// Every `rule_id` value in a JSON report, in document order.
pub fn json_rule_ids(json: &str) -> Vec<String> {
    const KEY: &str = "\"rule_id\": \"";
    let mut ids = Vec::new();
    let mut rest = json;
    while let Some(at) = rest.find(KEY) {
        rest = &rest[at + KEY.len()..];
        let end = rest.find('"').expect("unterminated rule_id value");
        ids.push(rest[..end].to_string());
        rest = &rest[end..];
    }
    ids
}

/// How many matches in a JSON report carry the given severity.
pub fn json_severity_count(json: &str, severity: &str) -> usize {
    json.matches(&format!("\"severity\": \"{severity}\""))
        .count()
}
