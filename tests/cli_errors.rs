//  Project:   macbash
//  File:      tests/cli_errors.rs
//  Purpose:   End-to-end tests for CLI validation failures and exit codes
//  Language:  Rust
//
//  License:   Apache-2.0
//  Copyright: (c) 2025-2026 HYPERI PTY LIMITED

mod common;

use predicates::prelude::*;

use common::{PORTABLE_SCRIPT, fixture, macbash, write_file};

#[test]
fn no_files_given_is_an_error() {
    macbash()
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "Error: no files specified. Use 'macbash --help' for usage",
        ))
        .stdout(predicate::str::is_empty());
}

#[test]
fn nonexistent_input_file_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("does-not-exist.sh");
    macbash()
        .arg(&missing)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("Error: file not found:"))
        .stderr(predicate::str::contains("does-not-exist.sh"));
}

#[test]
fn one_missing_file_among_several_still_fails() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("gone.sh");
    macbash()
        .arg(fixture("sample-linux.sh"))
        .arg(&missing)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("Error: file not found:"))
        .stdout(predicate::str::is_empty());
}

#[test]
fn write_and_output_together_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("fixed.sh");
    macbash()
        .arg("-w")
        .arg("-o")
        .arg(&out)
        .arg(fixture("sample-linux.sh"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "Error: cannot use both -w (--write) and -o (--output)",
        ));
    assert!(
        !out.exists(),
        "nothing should be written when the flags conflict"
    );
}

#[test]
fn invalid_severity_is_rejected() {
    macbash()
        .args(["--severity", "bogus"])
        .arg(fixture("sample-linux.sh"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "Error: invalid severity \"bogus\": must be error, warning, or info",
        ));
}

#[test]
fn invalid_format_is_rejected() {
    macbash()
        .args(["--format", "xml"])
        .arg(fixture("sample-linux.sh"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "Error: invalid format \"xml\": must be text or json",
        ));
}

#[test]
fn output_path_must_be_a_directory_for_multiple_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let not_a_dir = write_file(&dir, "occupied.sh", PORTABLE_SCRIPT);
    macbash()
        .arg("-o")
        .arg(&not_a_dir)
        .arg(fixture("sample-linux.sh"))
        .arg(fixture("stress-test-fixer.sh"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "Error: output path must be a directory when processing multiple files",
        ));
    assert_eq!(
        std::fs::read_to_string(&not_a_dir).unwrap(),
        PORTABLE_SCRIPT,
        "the existing file must not be clobbered"
    );
}

#[test]
fn missing_config_file_is_reported_with_context() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nope.yaml");
    macbash()
        .arg("--config")
        .arg(&missing)
        .arg(fixture("sample-linux.sh"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "Error: loading custom rules: reading rules file",
        ))
        .stderr(predicate::str::contains("nope.yaml"))
        // The OS error appears exactly once. thiserror's #[source] plus a
        // Display string that also named {source} used to print it twice.
        .stderr(
            predicate::str::contains("No such file or directory")
                .count(1)
                .from_utf8(),
        );
}

#[test]
fn config_file_with_an_uncompilable_pattern_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let rules = write_file(
        &dir,
        "bad.yaml",
        "version: \"1.0\"\nrules:\n  - id: bad-rule\n    name: Bad Rule\n    pattern: '[unclosed'\n    severity: error\n",
    );
    macbash()
        .arg("--config")
        .arg(&rules)
        .arg(fixture("sample-linux.sh"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains("invalid rule bad-rule"))
        .stderr(predicate::str::contains("invalid pattern"));
}

#[test]
fn unknown_flag_is_rejected_by_the_parser() {
    // clap owns this path; it uses exit code 2, not our exit code 1.
    macbash()
        .arg("--not-a-flag")
        .arg(fixture("sample-linux.sh"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--not-a-flag"));
}

#[test]
fn help_lists_the_output_modes_and_exits_zero() {
    macbash()
        .arg("--help")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Output Modes:"))
        .stdout(predicate::str::contains("-w, --write"))
        .stdout(predicate::str::contains("-o, --output"))
        .stdout(predicate::str::contains("Exit Codes:"));
}
