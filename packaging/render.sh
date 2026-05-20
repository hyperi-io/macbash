#!/bin/sh
#  Project:   rust-cli-packaging (in-tree framework)
#  File:      packaging/render.sh
#  Purpose:   Render parameterised installer templates from packaging.env
#  Language:  Shell (POSIX)
#
#  License:   Apache-2.0
#  Copyright: (c) 2025-2026 HYPERI PTY LIMITED
#
#  Usage:     packaging/render.sh
#             PACKAGING_ENV=/path/to/other.env packaging/render.sh

set -eu

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
PROJECT_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
CONFIG="${PACKAGING_ENV:-$PROJECT_ROOT/packaging.env}"
TEMPLATES="$SCRIPT_DIR/templates"
DIST="$SCRIPT_DIR/dist"

if [ ! -f "$CONFIG" ]; then
    echo "render.sh: config not found: $CONFIG" >&2
    echo "  create packaging.env at the project root — see packaging/README.md" >&2
    exit 1
fi

if ! command -v envsubst >/dev/null 2>&1; then
    echo "render.sh: envsubst not found (install gettext)" >&2
    exit 1
fi

# shellcheck disable=SC1090
. "$CONFIG"

# Required keys. Fail loudly if any is unset or empty.
: "${PKG_NAME:?packaging.env must set PKG_NAME}"
: "${PKG_BINARY:?packaging.env must set PKG_BINARY}"
: "${PKG_DOWNLOAD_BASE:?packaging.env must set PKG_DOWNLOAD_BASE}"
: "${PKG_DEFAULT_DIR:?packaging.env must set PKG_DEFAULT_DIR}"
: "${PKG_USER_DIR:?packaging.env must set PKG_USER_DIR}"
: "${PKG_WINDOWS_DEFAULT_DIR:?packaging.env must set PKG_WINDOWS_DEFAULT_DIR}"

export PKG_NAME PKG_BINARY PKG_DOWNLOAD_BASE PKG_DEFAULT_DIR PKG_USER_DIR PKG_WINDOWS_DEFAULT_DIR

# envsubst whitelist — only these vars get substituted; everything else
# (including $HOME, $1, $PATH, $env:LOCALAPPDATA in install.ps1) survives.
WHITELIST='${PKG_NAME} ${PKG_BINARY} ${PKG_DOWNLOAD_BASE} ${PKG_DEFAULT_DIR} ${PKG_USER_DIR} ${PKG_WINDOWS_DEFAULT_DIR}'

rm -rf "$DIST"
mkdir -p "$DIST"

count=0
for tmpl in "$TEMPLATES"/*; do
    [ -f "$tmpl" ] || continue
    name=$(basename "$tmpl")
    envsubst "$WHITELIST" < "$tmpl" > "$DIST/$name"
    case "$name" in
        *.sh) chmod +x "$DIST/$name" ;;
    esac
    count=$((count + 1))
done

echo "rendered $count template(s) -> $DIST"
ls -1 "$DIST"
