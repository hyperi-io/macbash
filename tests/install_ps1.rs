//  Project:   macbash
//  File:      tests/install_ps1.rs
//  Purpose:   End-to-end tests for the checksum gate in the rendered install.ps1
//  Language:  Rust
//
//  License:   Apache-2.0
//  Copyright: (c) 2026 HYPERI PTY LIMITED

// install.ps1 runs under PowerShell 7 (`pwsh`), which also runs on Linux and macOS.
// A host without pwsh cannot run it, so each test says so and returns.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use assert_cmd::Command;
use assert_cmd::assert::Assert;
use predicates::prelude::*;
use tempfile::TempDir;

use common::{OTHER_SHA256, Release, VERSION, serve, which};

/// The asset install.ps1 asks for, and the sums file the release pipeline writes for it.
const ASSET: &str = "macbash-windows-amd64.exe";
const PLATFORM_SUMS: &str = "macbash-windows-amd64.sha256";

/// The binary the test host publishes, and its SHA-256. install.ps1 never runs it.
const EXE: &str = "MZ macbash.exe test fixture\n";
const EXE_SHA256: &str = "b90fb180176de8051be70ed5d74ef117ea44f1d5da5246d0f9a7a5e284037646";

fn release_with_exe() -> Release {
    Release::default().file(ASSET, EXE)
}

/// A sums file laid out the way the release pipeline writes it: the .exe, then the zip.
fn platform_sums(hash: &str) -> String {
    format!("{hash}  {ASSET}\n{OTHER_SHA256}  macbash-windows-amd64.zip\n")
}

/// install.ps1 rendered against `base`, as packaging/render.sh would, written into `dir`.
fn render(dir: &TempDir, base: &str) -> PathBuf {
    let template_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("packaging/templates/install.ps1");
    let template =
        std::fs::read_to_string(&template_path).expect("reading the install.ps1 template");
    let bin_dir = install_dir(dir);
    let bin = bin_dir.to_str().expect("tempdir paths are utf-8");
    let rendered = template
        .replace("${PKG_NAME}", "macbash")
        .replace("${PKG_BINARY}", "macbash")
        .replace("${PKG_DOWNLOAD_BASE}", base)
        .replace("${PKG_WINDOWS_DEFAULT_DIR}", bin);
    assert!(
        !rendered.contains("${PKG_"),
        "install.ps1 uses a placeholder these tests do not render"
    );
    let script = dir.path().join("install.ps1");
    std::fs::write(&script, rendered).expect("writing the rendered install.ps1");
    script
}

fn install_dir(dir: &TempDir) -> PathBuf {
    dir.path().join("bin")
}

fn installed(dir: &TempDir) -> PathBuf {
    install_dir(dir).join("macbash.exe")
}

/// Run the installer against `release`, or `None` when this host has no pwsh.
fn install(release: Release) -> Option<(TempDir, Assert)> {
    let Some(pwsh) = which("pwsh") else {
        eprintln!("skipping: pwsh is not on PATH, so install.ps1 was not run");
        return None;
    };
    let dir = tempfile::tempdir().expect("creating a tempdir");
    let home = dir.path().join("home");
    let tmp = dir.path().join("tmp");
    std::fs::create_dir(&home).expect("creating HOME");
    std::fs::create_dir(&tmp).expect("creating the temp dir");
    let script = render(&dir, &serve(release));
    // install.ps1 reads TEMP and PROCESSOR_ARCHITECTURE, which only Windows sets. The
    // proxy bypass keeps a caller's proxy settings off the loopback download, and
    // NO_COLOR keeps ANSI escapes out of the error text the tests match.
    let run = Command::new(pwsh)
        .args(["-NoProfile", "-NonInteractive", "-File"])
        .arg(&script)
        .arg("-Dir")
        .arg(install_dir(&dir))
        .args(["-Version", VERSION])
        .env("HOME", &home)
        .env("TMPDIR", &tmp)
        .env("TEMP", &tmp)
        .env("PROCESSOR_ARCHITECTURE", "AMD64")
        .env("NO_PROXY", "127.0.0.1")
        .env("no_proxy", "127.0.0.1")
        .env("NO_COLOR", "1")
        .env("POWERSHELL_TELEMETRY_OPTOUT", "1")
        .env("POWERSHELL_UPDATECHECK", "Off")
        .timeout(Duration::from_secs(120))
        .assert();
    Some((dir, run))
}

/// PowerShell word-wraps an error to the console width and prefixes each continued line,
/// so a message is matched with its whitespace and wrap markers removed.
fn squashed(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace() && *c != '|')
        .collect()
}

fn stderr_says(message: &str) -> impl Predicate<[u8]> {
    let wanted = squashed(message);
    predicate::function(move |stderr: &[u8]| {
        squashed(&String::from_utf8_lossy(stderr)).contains(&wanted)
    })
}

fn assert_not_installed(dir: &TempDir) {
    assert!(
        !installed(dir).exists(),
        "install.ps1 must not install a binary it could not verify"
    );
}

/// The installer found no usable SHA-256 for the asset and refused.
fn assert_no_sha256(run: Assert) {
    run.code(1)
        .stderr(stderr_says(
            "install.ps1: no SHA-256 for macbash-windows-amd64.exe",
        ))
        .stderr(stderr_says("refusing to install an unverified binary"));
}

#[test]
fn installs_when_the_platform_checksum_matches() {
    let release = release_with_exe().file(PLATFORM_SUMS, platform_sums(EXE_SHA256));
    let Some((dir, run)) = install(release) else {
        return;
    };
    run.code(0).stdout(predicate::str::contains(format!(
        "Checksum verified: sha256 {EXE_SHA256}"
    )));
    let body = std::fs::read_to_string(installed(&dir)).expect("the binary is installed");
    assert_eq!(body, EXE);
}

#[test]
fn installs_from_the_combined_sums_file_when_no_platform_file_exists() {
    let combined = format!("{EXE_SHA256}  {ASSET}\n");
    let Some((dir, run)) = install(release_with_exe().file("checksums.sha256", combined)) else {
        return;
    };
    run.code(0).stdout(predicate::str::contains(format!(
        "Checksum verified: sha256 {EXE_SHA256}"
    )));
    assert!(installed(&dir).exists());
}

#[test]
fn installs_when_the_sums_file_marks_the_entry_binary() {
    // `sha256sum -b` and `shasum -b` write `<hash> *<name>`.
    let release = release_with_exe().file(PLATFORM_SUMS, format!("{EXE_SHA256} *{ASSET}\n"));
    let Some((dir, run)) = install(release) else {
        return;
    };
    run.code(0);
    assert!(installed(&dir).exists());
}

#[test]
fn refuses_when_no_sums_file_is_published() {
    let Some((dir, run)) = install(release_with_exe()) else {
        return;
    };
    assert_no_sha256(run);
    assert_not_installed(&dir);
}

#[test]
fn refuses_when_the_sums_files_do_not_name_this_asset() {
    // The binary's real hash sits against the zip's name, so matching the asset name as a
    // substring of a line would wrongly accept it.
    let release = release_with_exe()
        .file(
            PLATFORM_SUMS,
            format!("{EXE_SHA256}  macbash-windows-amd64.zip\n"),
        )
        .file(
            "checksums.sha256",
            format!("{EXE_SHA256}  macbash-linux-amd64\n"),
        );
    let Some((dir, run)) = install(release) else {
        return;
    };
    assert_no_sha256(run);
    assert_not_installed(&dir);
}

#[test]
fn refuses_an_asset_named_in_another_case() {
    // PowerShell compares strings case-insensitively unless told otherwise.
    let release = release_with_exe().file(
        PLATFORM_SUMS,
        format!("{EXE_SHA256}  MACBASH-WINDOWS-AMD64.EXE\n"),
    );
    let Some((dir, run)) = install(release) else {
        return;
    };
    assert_no_sha256(run);
    assert_not_installed(&dir);
}

#[test]
fn refuses_a_checksum_mismatch_even_when_the_combined_file_would_match() {
    let release = release_with_exe()
        .file(PLATFORM_SUMS, platform_sums(OTHER_SHA256))
        .file("checksums.sha256", format!("{EXE_SHA256}  {ASSET}\n"));
    let Some((dir, run)) = install(release) else {
        return;
    };
    run.code(1)
        .stderr(stderr_says(
            "install.ps1: checksum mismatch for macbash-windows-amd64.exe",
        ))
        .stderr(stderr_says(&format!("expected: {OTHER_SHA256}")))
        .stderr(stderr_says(&format!("actual: {EXE_SHA256}")));
    assert_not_installed(&dir);
}

#[test]
fn refuses_an_asset_listed_twice() {
    let release = release_with_exe().file(
        PLATFORM_SUMS,
        format!("{EXE_SHA256}  {ASSET}\n{OTHER_SHA256}  {ASSET}\n"),
    );
    let Some((dir, run)) = install(release) else {
        return;
    };
    assert_no_sha256(run);
    assert_not_installed(&dir);
}

#[test]
fn refuses_a_hash_that_is_not_64_hex_digits() {
    let release = release_with_exe().file(PLATFORM_SUMS, format!("b90fb180  {ASSET}\n"));
    let Some((dir, run)) = install(release) else {
        return;
    };
    assert_no_sha256(run);
    assert_not_installed(&dir);
}

#[test]
fn refuses_when_the_binary_download_fails() {
    let release = Release::default().file(PLATFORM_SUMS, platform_sums(EXE_SHA256));
    let Some((dir, run)) = install(release) else {
        return;
    };
    run.code(1)
        .stderr(stderr_says("install.ps1: download failed:"));
    assert_not_installed(&dir);
}
