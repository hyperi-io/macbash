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
URL="$DOWNLOAD_BASE/$VERSION_PATH/$ASSET"
SUMS_URL="$DOWNLOAD_BASE/$VERSION_PATH/checksums.sha256"

TMPDIR=$(mktemp -d "${TMPDIR:-/tmp}/${PKG_BINARY}-install.XXXXXX")
trap 'rm -rf "$TMPDIR"' EXIT

echo "Downloading $BINARY $VERSION_PATH ($OS-$ARCH)..."
if ! curl -fsSL "$URL" -o "$TMPDIR/$BINARY"; then
    echo "install.sh: download failed: $URL" >&2
    exit 1
fi

# Checksum verify if the sums file exists and references this asset
if curl -fsSL "$SUMS_URL" -o "$TMPDIR/checksums.sha256" 2>/dev/null; then
    if grep -q "$ASSET" "$TMPDIR/checksums.sha256"; then
        expected=$(grep "$ASSET" "$TMPDIR/checksums.sha256" | awk '{print $1}')
        # Portable SHA-256: shasum (macOS, BSD, Linux with perl-base) or
        # openssl as fallback. Both available on every system we ship to.
        if command -v shasum >/dev/null 2>&1; then
            actual=$(shasum -a 256 "$TMPDIR/$BINARY" | awk '{print $1}')
        elif command -v openssl >/dev/null 2>&1; then
            actual=$(openssl dgst -sha256 "$TMPDIR/$BINARY" | awk '{print $NF}')
        else
            actual=""
        fi
        if [ -n "$actual" ] && [ "$expected" != "$actual" ]; then
            echo "install.sh: checksum mismatch for $ASSET" >&2
            echo "  expected: $expected" >&2
            echo "  actual:   $actual" >&2
            exit 1
        fi
        if [ -n "$actual" ]; then
            echo "Checksum verified."
        fi
    fi
fi

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
