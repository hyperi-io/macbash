# macbash

[![CI](https://github.com/hyperi-io/macbash/actions/workflows/ci.yml/badge.svg)](https://github.com/hyperi-io/macbash/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-2024_edition-CE422B?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

Check and fix bash scripts so they work on macOS.
Detects GNU/Linux-specific bash constructs that won't work on macOS (BSD).
Auto-fixes many issues in place.

73 built-in rules covering GNU coreutils, bash 4+ features, and portability
papercuts. Custom rules via YAML.

## Install

```bash
# Universal POSIX installer (any Linux / macOS)
curl -fsSL https://downloads.hyperi.io/macbash/install.sh | sh                # system-wide (/usr/local/bin)
curl -fsSL https://downloads.hyperi.io/macbash/install.sh | sh -s -- --user   # user-local (~/.local/bin)

# Homebrew (macOS / Linux)
brew tap hyperi-io/tap && brew install macbash

# Debian / Ubuntu
curl -fsSL -O https://downloads.hyperi.io/macbash/latest/macbash_amd64.deb
sudo dpkg -i macbash_amd64.deb

# RHEL / Fedora
curl -fsSL -O https://downloads.hyperi.io/macbash/latest/macbash-1.x86_64.rpm
sudo rpm -i macbash-1.x86_64.rpm

# Scoop (Windows - EXPERIMENTAL, not in formal test path)
scoop bucket add hyperi https://github.com/hyperi-io/scoop-bucket
scoop install macbash

# PowerShell (Windows - EXPERIMENTAL)
irm https://downloads.hyperi.io/macbash/install.ps1 | iex
```

Pin a specific version by replacing `latest` with a tag, e.g.
`/macbash/v1.5.7/macbash-linux-amd64`.

Uninstall:

```bash
curl -fsSL https://downloads.hyperi.io/macbash/uninstall.sh | sh -s -- --all
```

## Usage

```bash
macbash script.sh                        # check (default)
macbash -w script.sh                     # fix in place
macbash -o fixed.sh script.sh            # fix to a new file
macbash -o ./fixed/ scripts/*.sh         # fix multiple files into a directory
macbash -w --dry-run script.sh           # preview fixes without writing
macbash --format json scripts/*.sh       # JSON output for CI
macbash --severity info script.sh        # include info-level findings
macbash --config custom-rules.yaml *.sh  # add custom rules
```

`-w` (auto-fix) is **experimental** - diff before committing rewritten
scripts; the fixer prints a banner reminding you.

A rule only auto-fixes when the result works on GNU and BSD alike. Where no
portable form exists (`date -d @EPOCH`, `xargs -r`), macbash reports the
problem and explains the trade-off rather than silently converting a working
Linux script into a macOS-only one.

Exit codes:

- `0` - no issues (or all fixed with `-w`/`-o`)
- `1` - errors found, or unfixable issues remained after a fix run

## What it catches

Run `macbash --severity info` to see every rule category. The headline
classes:

**GNU coreutils (error)** - `sed -i` without backup ext, `grep -P`,
`readlink -f`, `date -d`, `stat -c`, `xargs -r`, `find -printf`, `sort -V`,
`timeout`, and 40+ others.

**Bash 4+ features (error)** - macOS ships bash 3.2 due to GPLv3 licensing:
`declare -A`, `${var,,}`/`${var^^}`, `mapfile`/`readarray`, `|&`,
`${arr[-1]}`, `coproc`, etc.

**Portability (warning / info)** - `echo -e`, `echo -n`, `#!/bin/bash`
shebang, `gawk`, `pgrep -P` and friends.

Each finding includes:

- file, line, column, rule ID
- the offending source line
- a suggested fix (and, where deterministic, an auto-fix template)

Output is colourised on terminals and plain on pipes. `--format json` is
stable and matches the upstream Go schema field-for-field.

## Custom rules

```yaml
version: "1.0"
rules:
  - id: my-rule
    name: Custom check
    description: What this catches
    severity: warning            # error | warning | info
    pattern: 'some\s+pattern'    # POSIX ERE
    negative_pattern: 'exclude'  # optional - suppresses matches
    shebang_match: '^#!/bin/sh'  # optional - only apply when this matches the shebang
    fix_type: suggest            # suggest | replace | transform | function
    fix_template: "Use this instead"
    tags: [custom]
    test_cases:
      should_match:
        - "some matching line"
      should_not_match:
        - "exclude line"
```

Load with `macbash --config rules.yaml *.sh`. Custom rules merge on top of
the built-ins.

## CI integration

As a GitHub Action:

```yaml
- name: Check bash portability
  uses: hyperi-io/macbash@main
  with:
    paths: 'scripts/**/*.sh'
    severity: error
```

Inputs: `version` (default `latest`), `paths`, `severity`, `format`, `config`,
`fail-on-no-match` (default `true`, so a typo in `paths` fails the step rather
than passing an empty check). Linux and macOS runners only - Windows consumers
install from the Scoop bucket instead.

Or install it directly on any runner:

```yaml
- name: Check bash portability
  run: |
    curl -fsSL https://downloads.hyperi.io/macbash/install.sh | sh -s -- --user
    macbash scripts/*.sh
```

JSON output for tooling:

```json
{
  "total_issues": 5,
  "errors": 3,
  "warnings": 1,
  "infos": 1,
  "matches": [
    {
      "file": "script.sh",
      "line": 2,
      "column": 1,
      "rule_id": "sed-inplace-no-backup",
      "rule_name": "sed -i without backup extension",
      "severity": "error",
      "content": "sed -i 's/old/new/' file.txt",
      "matched": "sed -i 's",
      "fix": "sed -i.bak",
      "fix_type": "replace"
    }
  ]
}
```

## Development

```bash
git config core.hooksPath .githooks   # once per clone - enables the push guard
cargo build --release    # build
cargo test               # unit + integration tests
cargo clippy             # lint
hyperi-ci check          # full local pipeline (quality + test + build)
```

Project layout - Rust 2024 edition, single binary + library crate, no
unsafe code, embedded YAML rule corpus via `rust-embed`. Integration tests
drive the built binary over the deliberately-broken scripts in
`tests/fixtures/`. See
[hyperi-io/hyperi-ci](https://github.com/hyperi-io/hyperi-ci) for the CI
toolchain.

Releases are fully automated: a `Publish: true` trailer on any
`fix:`/`feat:` commit kicks off CI -> cross-build (darwin + windows) ->
package (linux tarballs + deb/rpm + homebrew + scoop).

## Documentation

| Doc | Read it when |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | You are changing the code |
| [docs/rules.md](docs/rules.md) | You are writing or overriding a rule |
| [docs/release.md](docs/release.md) | You want the release and packaging chain |
| [docs/decisions/](docs/decisions/) | You are about to re-litigate a design choice |
| [packaging/README.md](packaging/README.md) | You want the packaging framework for another Rust CLI |

## License

[Apache-2.0](LICENSE).
