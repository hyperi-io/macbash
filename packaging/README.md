# Rust CLI Packaging Framework

Parameterised installer scaffolding for Rust CLI projects published via
hyperi-ci. Drop the `packaging/` directory into any project, edit
`packaging.env` at the repo root, run `packaging/render.sh`, and ship
the rendered files.

## Status

| Platform | Status |
|---|---|
| Linux (amd64, arm64) | Supported — built and tested in CI |
| macOS (amd64, arm64) | Templates ready; binary builds pending hyperi-ci darwin support |
| **Windows (amd64)** | **EXPERIMENTAL — best effort.** Not in the formal CI test path. install.ps1 template ships, but Windows binaries are not yet produced by hyperi-ci. Planned phase 2 via `cargo-zigbuild` cross-compilation |

The Windows channel is provided so consumers can iterate, but it has
no CI gate behind it — expect rough edges until the formal test path
covers it.

## What it produces

Today: `install.sh` + `install.ps1`.

Both detect OS/arch, download the matching binary from the configured
R2 download base (or any HTTP host), verify a SHA-256 checksum if
available, and install to a system or user directory.

Future channels (Homebrew formula, nfpm deb/rpm, WinGet, Scoop) will
plug into the same template set — same `packaging.env`, additional
template files.

## How to use it in a new project

1. Copy `packaging/` into the new project's repo root.
2. Create `packaging.env` at the repo root. Minimum keys:

   ```sh
   PKG_NAME="myapp"
   PKG_BINARY="myapp"
   PKG_DOWNLOAD_BASE="https://downloads.example.com/myapp"
   PKG_DEFAULT_DIR="/usr/local/bin"
   PKG_USER_DIR='$HOME/.local/bin'   # single-quoted so $HOME stays literal
   PKG_WINDOWS_DEFAULT_DIR='$env:LOCALAPPDATA\Programs\myapp'
   ```

3. Add `packaging/dist/` to `.gitignore`.
4. Run `packaging/render.sh`. Rendered files land in `packaging/dist/`.
5. Upload the rendered `install.sh` and `install.ps1` to your download
   host. Users then run:

   ```sh
   curl -fsSL https://downloads.example.com/myapp/install.sh | sh
   ```

   ```powershell
   irm https://downloads.example.com/myapp/install.ps1 | iex
   ```

## Conventions the templates assume

The rendered installers expect binaries at this URL layout:

```
${PKG_DOWNLOAD_BASE}/${VERSION}/${PKG_BINARY}-${os}-${arch}        # unix
${PKG_DOWNLOAD_BASE}/${VERSION}/${PKG_BINARY}-${os}-${arch}.exe    # windows
${PKG_DOWNLOAD_BASE}/${VERSION}/checksums.sha256                   # optional
```

Where `${VERSION}` is either `latest` or a `vX.Y.Z` tag, and `os` is
one of `linux`, `darwin`, `windows`, and `arch` is `amd64` or `arm64`.

This matches what `hyperi-ci run publish` uploads to R2 today (with
`destinations_oss.binaries: r2-binaries`).

## Template engine

Pure POSIX shell + GNU `envsubst` (from `gettext`). No Python, no Rust,
no Node. `envsubst` is preinstalled on every Linux distro, macOS via
Homebrew (`brew install gettext`), and GitHub-hosted runners.

The render script whitelists which `${VAR}` references it substitutes,
so runtime variables like `$HOME`, `$1`, `$PATH` inside the templates
survive untouched.

## Files

| Path | Purpose |
|---|---|
| `packaging.env` (at repo root) | Per-project config. The only file consumers edit |
| `packaging/render.sh` | POSIX render script. Sources `packaging.env`, runs `envsubst` over each template |
| `packaging/templates/install.sh` | POSIX curl-pipe installer template |
| `packaging/templates/install.ps1` | PowerShell `irm \| iex` installer template |
| `packaging/dist/` | Rendered output (gitignored) |

## Adding a new template

Drop a new file into `packaging/templates/`. Reference any
`packaging.env` variable with `${PKG_*}` syntax. Add the variable to
the `WHITELIST` line in `render.sh`. Run `render.sh`.
