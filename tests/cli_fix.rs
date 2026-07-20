//  Project:   macbash
//  File:      tests/cli_fix.rs
//  Purpose:   End-to-end tests for fix mode -- -w, -o, --dry-run, legacy -f
//  Language:  Rust
//
//  License:   Apache-2.0
//  Copyright: (c) 2025-2026 HYPERI PTY LIMITED

mod common;

use predicates::prelude::*;

use common::{PORTABLE_SCRIPT, copy_fixture, finding_count, fixture, macbash, write_file};

#[test]
fn write_mode_rewrites_the_file_and_leaves_fewer_findings() {
    let dir = tempfile::tempdir().unwrap();
    let script = copy_fixture(&dir, "sample-linux.sh");
    let before = std::fs::read_to_string(&script).unwrap();
    let before_count = finding_count(&script);

    // Exit 1 because unfixable findings remain, not because the fix failed.
    macbash()
        .arg("-w")
        .arg(&script)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("fixes applied"))
        .stdout(predicate::str::contains("Total:"));

    let after = std::fs::read_to_string(&script).unwrap();
    assert_ne!(before, after, "-w must rewrite the file in place");
    let after_count = finding_count(&script);
    assert!(
        after_count < before_count,
        "re-scan after -w found {after_count} issues, was {before_count}"
    );
}

#[test]
fn write_mode_banner_goes_to_stderr_not_stdout() {
    // The EXPERIMENTAL warning must not pollute stdout, which CI may capture.
    let dir = tempfile::tempdir().unwrap();
    let script = copy_fixture(&dir, "sample-linux.sh");
    macbash()
        .arg("-w")
        .arg(&script)
        .assert()
        .stderr(predicate::str::contains("EXPERIMENTAL"))
        .stdout(predicate::str::contains("EXPERIMENTAL").not());
}

#[test]
fn write_mode_output_is_idempotent_on_a_second_pass() {
    let dir = tempfile::tempdir().unwrap();
    let script = copy_fixture(&dir, "sample-linux.sh");
    macbash().arg("-w").arg(&script).assert().code(1);
    let once = std::fs::read_to_string(&script).unwrap();
    macbash().arg("-w").arg(&script).assert().code(1);
    let twice = std::fs::read_to_string(&script).unwrap();
    assert_eq!(once, twice, "re-running -w must not keep mutating the file");
}

#[test]
fn write_mode_on_a_portable_script_copies_it_unchanged_and_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    let script = write_file(&dir, "clean.sh", PORTABLE_SCRIPT);
    macbash()
        .arg("-w")
        .arg(&script)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("(0 fixes applied, 0 unfixable)"))
        .stdout(predicate::str::contains(
            "Total: 0 fixes applied, 0 unfixable issues",
        ));
    assert_eq!(std::fs::read_to_string(&script).unwrap(), PORTABLE_SCRIPT);
}

#[test]
fn dry_run_previews_without_touching_the_original() {
    let dir = tempfile::tempdir().unwrap();
    let script = copy_fixture(&dir, "sample-linux.sh");
    let before = std::fs::read(&script).unwrap();

    macbash()
        .args(["-w", "--dry-run"])
        .arg(&script)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("[dry-run] Would fix"))
        .stdout(predicate::str::contains("--- Preview of changes ---"))
        .stdout(predicate::str::contains("--- End preview ---"))
        // The preview body is the rewritten script, so a known transform
        // must show up there even though nothing is written.
        .stdout(predicate::str::contains("grep -E '[0-9]+'"))
        // And a rule that deliberately does NOT auto-fix must come through
        // untouched: dropping xargs -r is correct on BSD but changes
        // behaviour on GNU, so it is reported, never rewritten.
        .stdout(predicate::str::contains("xargs -r rm"));

    assert_eq!(
        before,
        std::fs::read(&script).unwrap(),
        "--dry-run must leave the input byte-identical"
    );
}

#[test]
fn output_file_for_a_single_input_leaves_the_input_alone() {
    let dir = tempfile::tempdir().unwrap();
    let script = copy_fixture(&dir, "sample-linux.sh");
    let before = std::fs::read(&script).unwrap();
    let out = dir.path().join("fixed.sh");

    macbash()
        .arg("-o")
        .arg(&out)
        .arg(&script)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("fixed.sh"));

    assert!(predicate::path::exists().eval(&out), "-o file not written");
    assert_eq!(before, std::fs::read(&script).unwrap(), "input was mutated");
    assert_ne!(
        std::fs::read(&out).unwrap(),
        before,
        "-o output should differ from the unfixed input"
    );
}

#[test]
fn output_file_parent_directories_are_created() {
    let dir = tempfile::tempdir().unwrap();
    let script = copy_fixture(&dir, "sample-linux.sh");
    let out = dir.path().join("nested").join("deeper").join("fixed.sh");
    macbash().arg("-o").arg(&out).arg(&script).assert().code(1);
    assert!(predicate::path::is_file().eval(&out));
}

#[test]
fn output_dir_for_multiple_inputs_preserves_file_names() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("fixed");
    std::fs::create_dir(&out_dir).unwrap();

    macbash()
        .arg("-o")
        .arg(&out_dir)
        .arg(fixture("sample-linux.sh"))
        .arg(fixture("stress-test-fixer.sh"))
        .assert()
        .code(1);

    for name in ["sample-linux.sh", "stress-test-fixer.sh"] {
        assert!(
            predicate::path::is_file().eval(&out_dir.join(name)),
            "missing {name} in the output directory"
        );
    }
    // Fixtures are the input here, so prove they were not rewritten in place.
    for name in ["sample-linux.sh", "stress-test-fixer.sh"] {
        assert_ne!(
            std::fs::read(fixture(name)).unwrap(),
            std::fs::read(out_dir.join(name)).unwrap(),
            "{name} was copied verbatim, expected fixes"
        );
    }
}

#[test]
fn deprecated_fix_flag_warns_and_implies_write() {
    let dir = tempfile::tempdir().unwrap();
    let script = copy_fixture(&dir, "sample-linux.sh");
    let before = std::fs::read_to_string(&script).unwrap();

    macbash()
        .arg("-f")
        .arg(&script)
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "Warning: --fix is deprecated, use -w (--write) instead",
        ));

    assert_ne!(
        before,
        std::fs::read_to_string(&script).unwrap(),
        "-f must behave like -w and rewrite in place"
    );
}

#[test]
fn severity_filter_narrows_what_fix_mode_touches() {
    // Fixes are driven by the already-filtered match list, so raising the
    // severity floor must not fix anything a lower floor left alone.
    let dir = tempfile::tempdir().unwrap();
    let wide = copy_fixture(&dir, "sample-linux.sh");
    macbash()
        .args(["-w", "--severity", "info"])
        .arg(&wide)
        .assert()
        .code(1);

    let dir2 = tempfile::tempdir().unwrap();
    let narrow = copy_fixture(&dir2, "sample-linux.sh");
    macbash()
        .args(["-w", "--severity", "error"])
        .arg(&narrow)
        .assert()
        .code(1);

    assert!(
        finding_count(&wide) <= finding_count(&narrow),
        "--severity info should fix at least as much as --severity error"
    );
}
