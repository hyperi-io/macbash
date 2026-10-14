#!/bin/sh
#  Project:   ${PKG_NAME}
#  File:      install.sh (rendered)
#  Purpose:   Universal POSIX installer
#  Language:  Shell (POSIX)
#
#  License:   Apache-2.0
#  Copyright: (c) 2025-2026 HYPERI PTY LIMITED
#
#  Usage:
#    curl -fsSL ${PKG_DOWNLOAD_BASE}/install.sh | sh
#    curl -fsSL ${PKG_DOWNLOAD_BASE}/install.sh | sh -s -- --user
#    curl -fsSL ${PKG_DOWNLOAD_BASE}/install.sh | sh -s -- --version v1.2.3
#    curl -fsSL ${PKG_DOWNLOAD_BASE}/install.sh | sh -s -- --dir /opt/bin

set -eu

die() {
    echo "install.sh: $*" >&2
    exit 1
}

BINARY="${PKG_BINARY}"
DOWNLOAD_BASE="${PKG_DOWNLOAD_BASE}"
INSTALL_DIR="${PKG_DEFAULT_DIR}"
USER_INSTALL=0
VERSION="latest"

while [ $# -gt 0 ]; do
    case "$1" in
        --user)    USER_INSTALL=1; INSTALL_DIR="${PKG_USER_DIR}" ;;
        --system)  USER_INSTALL=0; INSTALL_DIR="${PKG_DEFAULT_DIR}" ;;
        --dir)     shift; INSTALL_DIR="$1" ;;
        --version) shift; VERSION="$1" ;;
        --help)
            cat <<EOF
Usage: install.sh [options]
  --user             Install to user dir (${PKG_USER_DIR})
  --system           Install to system dir (${PKG_DEFAULT_DIR}, default)
  --dir <path>       Install to a specific directory
  --version <tag>    Install a specific version (default: latest)
  --help             Show this help
EOF
            exit 0
            ;;
        *) echo "install.sh: unknown option: $1" >&2; exit 1 ;;
    esac
    shift
done

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$OS" in
    linux|darwin) ;;
    *) echo "install.sh: unsupported OS: $OS" >&2; exit 1 ;;
esac
case "$ARCH" in
    x86_64|amd64)   ARCH=amd64 ;;
    aarch64|arm64)  ARCH=arm64 ;;
    *) echo "install.sh: unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

if [ "$VERSION" = "latest" ]; then
    VERSION_PATH="latest"
else
    case "$VERSION" in
        v*) VERSION_PATH="$VERSION" ;;
        *)  VERSION_PATH="v$VERSION" ;;
    esac
fi

ASSET="$BINARY-$OS-$ARCH"
RELEASE_URL="$DOWNLOAD_BASE/$VERSION_PATH"

# Nothing installs unverified, so a host that cannot hash stops before downloading.
if command -v sha256sum >/dev/null 2>&1; then
    HASHER=sha256sum
elif command -v shasum >/dev/null 2>&1; then
    HASHER=shasum
elif command -v openssl >/dev/null 2>&1; then
    HASHER=openssl
else
    die "no SHA-256 tool found (need sha256sum, shasum or openssl), refusing to install an unverified binary"
fi

sha256_of() {
    case "$HASHER" in
        sha256sum) sha256sum "$1" ;;
        shasum)    shasum -a 256 "$1" ;;
        openssl)   openssl dgst -sha256 -r "$1" ;;
    esac | awk '{ print tolower($1) }'
}

# HTTP errors fail the download, and a redirect may not leave HTTPS.
fetch() {
    curl -fsSL --proto-redir '=https' --tlsv1.2 --connect-timeout 30 -o "$2" "$1"
}

# The SHA-256 that sums file $1 records for exactly the file named $2, or nothing
# when that entry is missing, malformed or listed more than once.
sum_for() {
    awk -v name="$2" '
        { file = $2; sub(/^\*/, "", file) }
        NF == 2 && file == name { count++; hash = tolower($1) }
        END { if (count == 1 && length(hash) == 64 && hash !~ /[^0-9a-f]/) print hash }
    ' "$1"
}

TMPDIR=$(mktemp -d "${TMPDIR:-/tmp}/${PKG_BINARY}-install.XXXXXX")
trap 'rm -rf "$TMPDIR"' EXIT

echo "Downloading $BINARY $VERSION_PATH ($OS-$ARCH)..."
fetch "$RELEASE_URL/$ASSET" "$TMPDIR/$BINARY" || die "download failed: $RELEASE_URL/$ASSET"

# Every release publishes <asset>.sha256, and checksums.sha256 is the older combined
# file. The first one that names this asset decides, so a mismatch never falls through.
expected=""
for sums in "$ASSET.sha256" checksums.sha256; do
    if fetch "$RELEASE_URL/$sums" "$TMPDIR/$sums"; then
        expected=$(sum_for "$TMPDIR/$sums" "$ASSET")
    fi
    if [ -n "$expected" ]; then
        break
    fi
done
if [ -z "$expected" ]; then
    die "no SHA-256 for $ASSET in $RELEASE_URL/$ASSET.sha256 or $RELEASE_URL/checksums.sha256, refusing to install an unverified binary"
fi

actual=$(sha256_of "$TMPDIR/$BINARY")
[ -n "$actual" ] || die "could not compute the SHA-256 of $ASSET with $HASHER"
if [ "$expected" != "$actual" ]; then
    echo "install.sh: checksum mismatch for $ASSET, refusing to install" >&2
    echo "  expected: $expected" >&2
    echo "  actual:   $actual" >&2
    exit 1
fi
echo "Checksum verified: sha256 $actual"

chmod +x "$TMPDIR/$BINARY"

if [ "$USER_INSTALL" = 1 ] || [ -w "$INSTALL_DIR" ] || [ -w "$(dirname "$INSTALL_DIR")" ]; then
    mkdir -p "$INSTALL_DIR"
    mv "$TMPDIR/$BINARY" "$INSTALL_DIR/$BINARY"
else
    echo "Installing to $INSTALL_DIR (requires sudo)..."
    sudo mkdir -p "$INSTALL_DIR"
    sudo mv "$TMPDIR/$BINARY" "$INSTALL_DIR/$BINARY"
fi

if [ -x "$INSTALL_DIR/$BINARY" ]; then
    INSTALLED=$("$INSTALL_DIR/$BINARY" --version 2>&1 | head -1)
    echo "Installed: $INSTALLED ($INSTALL_DIR/$BINARY)"
else
    echo "Installed to $INSTALL_DIR/$BINARY"
fi

case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        echo ""
        echo "Note: $INSTALL_DIR is not in your PATH. Add to your shell profile:"
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        ;;
esac
