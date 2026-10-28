//  Project:   macbash
//  File:      tests/cli_check.rs
//  Purpose:   End-to-end tests for check mode -- reporting, formats, filters
//  Language:  Rust
//
//  License:   Apache-2.0
//  Copyright: (c) 2025-2026 HYPERI PTY LIMITED

mod common;

use std::collections::HashSet;

use predicates::prelude::*;

use common::{
    CUSTOM_RULES_INPUT, CUSTOM_RULES_YAML, PORTABLE_SCRIPT, fixture, json_rule_ids,
    json_severity_count, json_uint, macbash, stdout_of, write_file,
};

#[test]
fn check_mode_reports_findings_and_exits_one() {
    macbash()
        .arg(fixture("sample-linux.sh"))
        .assert()
        .code(1)
        .stdout(predicate::str::contains("sed-inplace-no-backup"))
        .stdout(predicate::str::contains("grep-perl-regex"))
        .stdout(predicate::str::contains("xargs-no-run-if-empty"))
        .stdout(predicate::str::contains("ERROR"))
        .stdout(predicate::str::contains("Found"));
}

#[test]
fn check_mode_headers_each_file_and_prints_a_summary() {
    // Running from inside the tempdir keeps the report header a bare file
    // name, so this stays readable without matching an absolute path.
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir, "probe.sh", "#!/usr/bin/env bash\necho -e \"hi\"\n");
    macbash()
        .current_dir(dir.path())
        .arg("probe.sh")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("probe.sh"))
        .stdout(predicate::str::contains("WARNING [echo-escape]"))
        .stdout(predicate::str::contains("Found 1 warning(s) in 1 file(s)"));
}

#[test]
fn check_mode_warnings_only_exit_zero() {
    // has_errors() gates the exit code, so a warning-only script is still 0.
    let dir = tempfile::tempdir().unwrap();
    let script = write_file(&dir, "warn.sh", "#!/usr/bin/env bash\necho -e \"hi\"\n");
    macbash().arg(&script).assert().code(0);
}

#[test]
fn portable_script_exits_zero_and_reports_no_issues() {
    let dir = tempfile::tempdir().unwrap();
    let script = write_file(&dir, "clean.sh", PORTABLE_SCRIPT);
    macbash()
        .args(["--severity", "info"])
        .arg(&script)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("No issues found."));
}

#[test]
fn piped_output_carries_no_ansi_escapes() {
    // Colour is gated on stdout being a terminal; assert_cmd always pipes.
    macbash()
        .arg(fixture("sample-linux.sh"))
        .assert()
        .stdout(predicate::str::contains("\u{1b}[").not());
}

#[test]
fn multiple_inputs_are_all_reported() {
    macbash()
        .arg(fixture("deploy-script.sh"))
        .arg(fixture("system-monitor.sh"))
        .assert()
        .code(1)
        .stdout(predicate::str::contains("deploy-script.sh"))
        .stdout(predicate::str::contains("system-monitor.sh"))
        .stdout(predicate::str::contains("in 2 file(s)"));
}

#[test]
fn json_report_has_the_documented_top_level_fields() {
    let json = stdout_of(
        macbash()
            .args(["--format", "json"])
            .arg(fixture("log-analyzer.sh")),
    );
    assert!(
        json.trim_start().starts_with('{') && json.trim_end().ends_with('}'),
        "json report is not a single object:\n{json}"
    );
    for key in ["total_issues", "errors", "warnings", "infos", "matches"] {
        assert!(
            json.contains(&format!("\"{key}\"")),
            "missing top-level key {key:?} in:\n{json}"
        );
    }
}

#[test]
fn json_report_counts_agree_with_the_matches_array() {
    let json = stdout_of(
        macbash()
            .args(["--format", "json", "--severity", "info"])
            .arg(fixture("log-analyzer.sh")),
    );
    let ids = json_rule_ids(&json);
    let total = json_uint(&json, "total_issues");
    let errors = json_uint(&json, "errors");
    let warnings = json_uint(&json, "warnings");
    let infos = json_uint(&json, "infos");

    assert!(total > 0, "fixture must produce findings");
    assert_eq!(total, ids.len(), "total_issues != matches array length");
    assert_eq!(errors, json_severity_count(&json, "error"));
    assert_eq!(warnings, json_severity_count(&json, "warning"));
    assert_eq!(infos, json_severity_count(&json, "info"));
    assert_eq!(
        total,
        errors + warnings + infos,
        "every match must be counted in exactly one severity bucket"
    );
}

#[test]
fn json_report_for_a_portable_script_is_an_empty_result() {
    let dir = tempfile::tempdir().unwrap();
    let script = write_file(&dir, "clean.sh", PORTABLE_SCRIPT);
    let json = stdout_of(
        macbash()
            .args(["--format", "json", "--severity", "info"])
            .arg(&script),
    );
    assert_eq!(json_uint(&json, "total_issues"), 0);
    assert!(json_rule_ids(&json).is_empty());
    assert!(json.contains("\"matches\": []"), "got:\n{json}");
}

#[test]
fn severity_filters_form_nested_subsets() {
    let ids_at = |severity: &str| -> HashSet<String> {
        let json = stdout_of(
            macbash()
                .args(["--format", "json", "--severity", severity])
                .arg(fixture("sample-linux.sh")),
        );
        json_rule_ids(&json).into_iter().collect()
    };
    let errors = ids_at("error");
    let warnings = ids_at("warning");
    let infos = ids_at("info");

    assert!(!errors.is_empty(), "fixture must produce error findings");
    assert!(
        errors.is_subset(&warnings),
        "error findings must survive the warning filter"
    );
    assert!(
        warnings.is_subset(&infos),
        "warning findings must survive the info filter"
    );
    // The fixture carries at least one rule at each severity, so each widening
    // of the filter must actually admit something new.
    assert!(errors.len() < warnings.len());
    assert!(warnings.len() < infos.len());
}

#[test]
fn severity_short_flag_matches_the_long_flag() {
    let long = stdout_of(
        macbash()
            .args(["--format", "json", "--severity", "error"])
            .arg(fixture("sample-linux.sh")),
    );
    let short = stdout_of(
        macbash()
            .args(["--format", "json", "-s", "error"])
            .arg(fixture("sample-linux.sh")),
    );
    assert_eq!(long, short);
}

#[test]
fn custom_config_rule_fires_alongside_the_builtins() {
    let dir = tempfile::tempdir().unwrap();
    let rules = write_file(&dir, "rules.yaml", CUSTOM_RULES_YAML);
    let script = write_file(&dir, "input.sh", CUSTOM_RULES_INPUT);
    let json = stdout_of(
        macbash()
            .args(["--format", "json"])
            .arg("--config")
            .arg(&rules)
            .arg(&script),
    );
    let ids = json_rule_ids(&json);
    assert!(
        ids.iter().any(|id| id == "custom-apt-get"),
        "custom rule did not fire:\n{json}"
    );
    assert!(
        ids.iter().any(|id| id == "echo-escape"),
        "builtin rules must still apply when --config is given:\n{json}"
    );
}

#[test]
fn custom_config_rule_is_absent_without_the_flag() {
    // Guards the previous test: the hit must come from the config, not a
    // builtin that happens to cover apt-get.
    let dir = tempfile::tempdir().unwrap();
    let script = write_file(&dir, "input.sh", CUSTOM_RULES_INPUT);
    let json = stdout_of(macbash().args(["--format", "json"]).arg(&script));
    assert!(!json_rule_ids(&json).iter().any(|id| id == "custom-apt-get"));
}

#[test]
fn version_flag_prints_macbash_and_semver() {
    let expected = format!("macbash {}", env!("CARGO_PKG_VERSION"));
    macbash()
        .arg("--version")
        .assert()
        .code(0)
        .stdout(predicate::str::starts_with(expected))
        .stdout(predicate::str::is_match(r"^macbash \d+\.\d+\.\d+\n").unwrap());
}

#[test]
fn version_short_flag_matches_the_long_flag() {
    assert_eq!(
        stdout_of(macbash().arg("-v")),
        stdout_of(macbash().arg("--version"))
    );
}

#[test]
fn text_output_prefix_is_line_then_column_once() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir, "probe.sh", "#!/usr/bin/env bash\necho -e \"hi\"\n");
    macbash()
        .current_dir(dir.path())
        .arg("probe.sh")
        .assert()
        .code(0)
        .stdout(predicate::str::is_match(r"(?m)^  2:\d+: (ERROR|WARNING|INFO) ").unwrap());
}
