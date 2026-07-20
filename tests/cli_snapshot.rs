//  Project:   macbash
//  File:      tests/cli_snapshot.rs
//  Purpose:   insta snapshots of the two byte-stable CLI outputs
//  Language:  Rust
//
//  License:   Apache-2.0
//  Copyright: (c) 2025-2026 HYPERI PTY LIMITED

// Snapshots are used only where the output is deterministic. Each test runs
// the binary with its cwd set to the tempdir and passes a relative file name,
// so no absolute tempdir path reaches the output. Everything path-bearing or
// build-stamped (the --version banner, fix-mode summaries over fixtures) is
// asserted with predicates in the other test files instead.

mod common;

use common::{macbash, stdout_of, write_file};

/// Two findings, one error and one warning, both with ASCII-only rule text.
const MINIMAL_SCRIPT: &str =
    "#!/usr/bin/env bash\nfind . -name '*.tmp' | xargs -r rm\necho -e \"done\"\n";

/// Only replace/transform rules, so the whole file is rewritten by the fixer.
const FIXABLE_SCRIPT: &str = concat!(
    "#!/usr/bin/env bash\n",
    "find . -name '*.tmp' | xargs -r rm\n",
    "grep -P '\\d+' numbers.txt\n",
    "date -d @1609459200\n",
    "echo -e \"done\"\n",
);

#[test]
fn json_report_for_a_minimal_script() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir, "probe.sh", MINIMAL_SCRIPT);
    let out = stdout_of(
        macbash()
            .current_dir(dir.path())
            .args(["--format", "json", "--severity", "info"])
            .arg("probe.sh"),
    );
    insta::assert_snapshot!(out);
}

#[test]
fn dry_run_preview_of_the_fixer_output() {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir, "probe.sh", FIXABLE_SCRIPT);
    let out = stdout_of(
        macbash()
            .current_dir(dir.path())
            .args(["-w", "--dry-run", "--severity", "info"])
            .arg("probe.sh"),
    );
    insta::assert_snapshot!(out);
}
