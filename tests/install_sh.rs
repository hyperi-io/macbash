//  Project:   macbash
//  File:      tests/install_sh.rs
//  Purpose:   End-to-end tests for the checksum gate in the rendered install.sh
//  Language:  Rust
//
//  License:   Apache-2.0
//  Copyright: (c) 2026 HYPERI PTY LIMITED

// install.sh is a POSIX sh script driven by curl, so it is only tested on unix.
#![cfg(unix)]

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use assert_cmd::Command;
use assert_cmd::assert::Assert;
use predicates::prelude::*;
use tempfile::TempDir;

use common::{OTHER_SHA256, Release, VERSION, serve, which};

/// The binary the test host publishes, and its SHA-256.
const BINARY: &str = "#!/bin/sh\necho 'macbash 9.9.9 (test fixture)'\n";
const BINARY_SHA256: &str = "7146ea8f8b6070cbac667c0a82b4f1262ea9520a5a6659ed36dcb6852ebc2753";

/// Every asset install.sh can ask for, so the same release serves any unix host.
const ASSETS: [&str; 4] = [
    "macbash-linux-amd64",
    "macbash-linux-arm64",
    "macbash-darwin-amd64",
    "macbash-darwin-arm64",
];

/// Everything install.sh runs on the install path apart from a SHA-256 tool.
const BASE_TOOLS: [&str; 11] = [
    "awk", "chmod", "curl", "dirname", "head", "mkdir", "mktemp", "mv", "rm", "tr", "uname",
];

impl Release {
    fn with_binaries() -> Self {
        ASSETS.iter().fold(Self::default(), |release, asset| {
            release.file(asset, BINARY)
        })
    }

    /// A `<asset>.sha256` per asset, laid out the way the release pipeline writes them.
    fn per_asset_sums(self, hash: &str) -> Self {
        ASSETS.iter().fold(self, |release, asset| {
            let body = format!("{hash}  {asset}\n{OTHER_SHA256}  {asset}.tar.gz\n");
            release.file(&format!("{asset}.sha256"), body)
        })
    }
}

/// install.sh rendered against `base`, as packaging/render.sh would, written into `dir`.
fn render(dir: &TempDir, base: &str) -> PathBuf {
    let template_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("packaging/templates/install.sh");
    let template =
        std::fs::read_to_string(&template_path).expect("reading the install.sh template");
    let bin_dir = install_dir(dir);
    let bin = bin_dir.to_str().expect("tempdir paths are utf-8");
    let rendered = template
        .replace("${PKG_NAME}", "macbash")
        .replace("${PKG_BINARY}", "macbash")
        .replace("${PKG_DOWNLOAD_BASE}", base)
        .replace("${PKG_DEFAULT_DIR}", bin)
        .replace("${PKG_USER_DIR}", bin);
    assert!(
        !rendered.contains("${PKG_"),
        "install.sh uses a placeholder these tests do not render"
    );
    let script = dir.path().join("install.sh");
    std::fs::write(&script, rendered).expect("writing the rendered install.sh");
    script
}

fn install_dir(dir: &TempDir) -> PathBuf {
    dir.path().join("bin")
}

fn installed(dir: &TempDir) -> PathBuf {
    install_dir(dir).join("macbash")
}

/// A PATH holding only the base tools plus `hashers`, linked from the real PATH.
fn restricted_path(dir: &TempDir, hashers: &[&str]) -> String {
    let tools = dir.path().join("tools");
    std::fs::create_dir(&tools).expect("creating the tools dir");
    for tool in BASE_TOOLS.iter().chain(hashers) {
        let real = which(tool).unwrap_or_else(|| panic!("{tool} must be on PATH"));
        std::os::unix::fs::symlink(real, tools.join(tool)).expect("linking a tool");
    }
    tools.to_str().expect("tempdir paths are utf-8").to_string()
}

/// Run the installer against `release`. `hashers` narrows PATH to those SHA-256
/// tools plus the base tools, and `None` keeps the caller's PATH.
fn install(release: Release, hashers: Option<&[&str]>) -> (TempDir, Assert) {
    let dir = tempfile::tempdir().expect("creating a tempdir");
    let home = dir.path().join("home");
    let tmp = dir.path().join("tmp");
    std::fs::create_dir(&home).expect("creating HOME");
    std::fs::create_dir(&tmp).expect("creating TMPDIR");
    let path = match hashers {
        Some(hashers) => restricted_path(&dir, hashers),
        None => std::env::var("PATH").expect("PATH is set"),
    };
    let script = render(&dir, &serve(release));
    // A cleared environment keeps the caller's proxy settings and ~/.curlrc out of the run.
    let run = Command::new("/bin/sh")
        .arg(&script)
        .arg("--user")
        .arg("--dir")
        .arg(install_dir(&dir))
        .args(["--version", VERSION])
        .env_clear()
        .env("PATH", path)
        .env("HOME", &home)
        .env("TMPDIR", &tmp)
        .timeout(Duration::from_secs(60))
        .assert();
    (dir, run)
}

fn assert_not_installed(dir: &TempDir) {
    assert!(
        !installed(dir).exists(),
        "install.sh must not install a binary it could not verify"
    );
}

#[test]
fn installs_when_the_per_asset_checksum_matches() {
    let (dir, run) = install(Release::with_binaries().per_asset_sums(BINARY_SHA256), None);
    run.code(0)
        .stdout(predicate::str::contains(format!(
            "Checksum verified: sha256 {BINARY_SHA256}"
        )))
        .stdout(predicate::str::contains(
            "Installed: macbash 9.9.9 (test fixture)",
        ));
    let mode = std::fs::metadata(installed(&dir))
        .expect("the binary is installed")
        .permissions()
        .mode();
    assert_ne!(mode & 0o111, 0, "the installed binary must be executable");
}

#[test]
fn installs_from_the_combined_sums_file_when_no_per_asset_file_exists() {
    let combined: String = ASSETS
        .iter()
        .map(|asset| format!("{BINARY_SHA256}  {asset}\n"))
        .collect();
    let (dir, run) = install(
        Release::with_binaries().file("checksums.sha256", combined),
        None,
    );
    run.code(0).stdout(predicate::str::contains(format!(
        "Checksum verified: sha256 {BINARY_SHA256}"
    )));
    assert!(installed(&dir).exists());
}

#[test]
fn refuses_when_no_sums_file_is_published() {
    let (dir, run) = install(Release::with_binaries(), None);
    run.code(1)
        .stderr(predicate::str::contains(
            "install.sh: no SHA-256 for macbash-",
        ))
        .stderr(predicate::str::contains(
            "refusing to install an unverified binary",
        ));
    assert_not_installed(&dir);
}

#[test]
fn refuses_when_the_sums_files_do_not_name_this_asset() {
    // The binary's real hash sits against the archive name, so matching the asset
    // name as a substring of a line would wrongly accept it.
    let release = ASSETS
        .iter()
        .fold(Release::with_binaries(), |release, asset| {
            release.file(
                &format!("{asset}.sha256"),
                format!("{BINARY_SHA256}  {asset}.tar.gz\n"),
            )
        })
        .file(
            "checksums.sha256",
            format!("{BINARY_SHA256}  macbash-windows-amd64.exe\n"),
        );
    let (dir, run) = install(release, None);
    run.code(1).stderr(predicate::str::contains(
        "install.sh: no SHA-256 for macbash-",
    ));
    assert_not_installed(&dir);
}

#[test]
fn refuses_a_checksum_mismatch_even_when_the_combined_file_would_match() {
    let combined: String = ASSETS
        .iter()
        .map(|asset| format!("{BINARY_SHA256}  {asset}\n"))
        .collect();
    let release = Release::with_binaries()
        .per_asset_sums(OTHER_SHA256)
        .file("checksums.sha256", combined);
    let (dir, run) = install(release, None);
    run.code(1)
        .stderr(predicate::str::contains(
            "install.sh: checksum mismatch for macbash-",
        ))
        .stderr(predicate::str::contains(format!(
            "expected: {OTHER_SHA256}"
        )))
        .stderr(predicate::str::contains(format!(
            "actual:   {BINARY_SHA256}"
        )));
    assert_not_installed(&dir);
}

#[test]
fn refuses_an_asset_listed_twice() {
    let release = ASSETS
        .iter()
        .fold(Release::with_binaries(), |release, asset| {
            release.file(
                &format!("{asset}.sha256"),
                format!("{BINARY_SHA256}  {asset}\n{OTHER_SHA256}  {asset}\n"),
            )
        });
    let (dir, run) = install(release, None);
    run.code(1).stderr(predicate::str::contains(
        "install.sh: no SHA-256 for macbash-",
    ));
    assert_not_installed(&dir);
}

#[test]
fn refuses_when_the_binary_download_fails() {
    let (dir, run) = install(Release::default().per_asset_sums(BINARY_SHA256), None);
    run.code(1)
        .stderr(predicate::str::contains("install.sh: download failed:"));
    assert_not_installed(&dir);
}

#[test]
fn refuses_before_downloading_when_the_host_has_no_sha256_tool() {
    let (dir, run) = install(
        Release::with_binaries().per_asset_sums(BINARY_SHA256),
        Some(&[]),
    );
    run.code(1)
        .stdout(predicate::str::contains("Downloading").not())
        .stderr(predicate::str::contains(
            "install.sh: no SHA-256 tool found (need sha256sum, shasum or openssl)",
        ));
    assert_not_installed(&dir);
}

#[test]
fn verifies_with_each_supported_sha256_tool() {
    let available: Vec<&str> = ["sha256sum", "shasum", "openssl"]
        .into_iter()
        .filter(|hasher| which(hasher).is_some())
        .collect();
    assert!(
        !available.is_empty(),
        "this host has no SHA-256 tool, so install.sh cannot install anything"
    );
    for hasher in available {
        let (dir, run) = install(
            Release::with_binaries().per_asset_sums(BINARY_SHA256),
            Some(&[hasher]),
        );
        run.code(0).stdout(predicate::str::contains(format!(
            "Checksum verified: sha256 {BINARY_SHA256}"
        )));
        assert!(installed(&dir).exists(), "{hasher} path did not install");
    }
}
