# macbash

[![CI](https://github.com/hypersec-io/macbash/actions/workflows/ci.yml/badge.svg)](https://github.com/hypersec-io/macbash/actions/workflows/ci.yml)
[![Go](https://img.shields.io/badge/Go-1.23+-00ADD8?logo=go&logoColor=white)](https://go.dev/)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

Check and fix bash scripts so they work on macOS.  
Detects GNU/Linux-specific bash constructs that won't work on macOS. Can auto-fix many issues.  

## Install

```bash
# Quick install
curl -fsSL https://raw.githubusercontent.com/hypersec-io/macbash/main/packaging/install.sh | sh

# Homebrew
brew tap hypersec-io/tools && brew install macbash

# Debian/Ubuntu
curl -LO https://github.com/hypersec-io/macbash/releases/latest/download/macbash_1.0.0_amd64.deb
sudo dpkg -i macbash_1.0.0_amd64.deb

# RHEL/Fedora
curl -LO https://github.com/hypersec-io/macbash/releases/latest/download/macbash-1.0.0-1.x86_64.rpm
sudo rpm -i macbash-1.0.0-1.x86_64.rpm

# From source
go install github.com/hypersec-io/macbash/cmd/macbash@latest
```

## Usage

```bash
macbash script.sh                        # Check for issues
macbash -w script.sh                     # Fix and overwrite in-place
macbash -o fixed.sh script.sh            # Fix to new file
macbash --format json scripts/*.sh       # JSON output for CI
macbash --config custom-rules.yaml *.sh  # Custom rules
```

Exit codes: 0 = clean, 1 = errors, 2 = warnings only

## What It Catches

### GNU Coreutils (Error)

| Issue | Problem | Fix |
|-------|---------|-----|
| `sed -i` | BSD requires backup extension | `sed -i ''` or `sed -i.bak` |
| `grep -P` | Perl regex is GNU-only | `grep -E` (auto-converts simple patterns) |
| `readlink -f` | GNU-only | `cd/pwd -P` combo |
| `date -d` | GNU-only date parsing | `date -j -f` on BSD |
| `stat -c` | GNU format option | `stat -f` on BSD |
| `xargs -r` | GNU no-run-if-empty | Remove (BSD default) |
| `find -printf` | GNU-only | `-exec stat` |
| `sort -V` | GNU version sort | Custom function |
| `timeout` | GNU coreutils | `gtimeout` via Homebrew |

### Bash 4+ Features (Error)

macOS ships bash 3.2 (GPLv3 licensing):

| Feature | Version | Alternative |
|---------|---------|-------------|
| `declare -A` | 4.0+ | Indexed arrays |
| `${var,,}` `${var^^}` | 4.0+ | `tr` |
| `mapfile`/`readarray` | 4.0+ | while-read loop |
| `\|&` | 4.0+ | `2>&1 \|` |
| `${arr[-1]}` | 4.3+ | `${arr[${#arr[@]}-1]}` |

### Portability (Warning/Info)

| Issue | Severity | Fix |
|-------|----------|-----|
| `echo -e` | Warning | `printf "%b"` |
| `echo -n` | Info | `printf "%s"` |
| `#!/bin/bash` | Info | `#!/usr/bin/env bash` |
| `gawk` | Warning | `awk` |

## Custom Rules

```yaml
version: "1.0"
rules:
  - id: my-rule
    name: "Custom check"
    description: "What this catches"
    severity: warning
    pattern: 'some\s+pattern'
    negative_pattern: 'exclude\s+this'
    fix_type: suggest
    fix_template: "Use this instead"
    tags: [custom]
```

## CI Integration

```yaml
- name: Check bash portability
  run: |
    go install github.com/hypersec-io/macbash/cmd/macbash@latest
    macbash scripts/*.sh
```

JSON output for reporting:

```json
{
  "total_issues": 2,
  "errors": 1,
  "warnings": 1,
  "matches": [
    {
      "file": "script.sh",
      "line": 5,
      "rule_id": "grep-perl-regex",
      "severity": "error",
      "content": "grep -P '\\d+' file.txt",
      "fix": "grep -E or perl -ne"
    }
  ]
}
```

## Development

```bash
go build ./cmd/macbash
go test ./...
golangci-lint run
```

## License

Apache-2.0
