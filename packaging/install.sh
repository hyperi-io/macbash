#!/bin/sh
# Project:   macbash
# File:      packaging/install.sh
# Purpose:   Universal installer script for macbash
# Language:  Shell (POSIX)
#
# License:   Apache-2.0
# Copyright: (c) 2025 HyperSec Pty Ltd
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/hypersec-io/macbash/main/packaging/install.sh | sh
#   curl -fsSL ... | sh -s -- --user    # Install to ~/.local/bin
#   curl -fsSL ... | sh -s -- --system  # Install to /usr/local/bin (default, requires sudo)

set -e

REPO="hypersec-io/macbash"
INSTALL_DIR="/usr/local/bin"
USER_INSTALL=false

# Parse arguments
while [ $# -gt 0 ]; do
    case "$1" in
        --user)
            USER_INSTALL=true
            INSTALL_DIR="$HOME/.local/bin"
            ;;
        --system)
            USER_INSTALL=false
            INSTALL_DIR="/usr/local/bin"
            ;;
        --dir)
            shift
            INSTALL_DIR="$1"
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
    shift
done

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
    linux)
        OS="linux"
        ;;
    darwin)
        OS="darwin"
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64)
        ARCH="amd64"
        ;;
    aarch64|arm64)
        ARCH="arm64"
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

echo "Detected: $OS-$ARCH"

# Get latest release version
echo "Fetching latest release..."
LATEST_VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/' || echo "")

if [ -z "$LATEST_VERSION" ]; then
    echo "Failed to get latest version. Check your internet connection."
    exit 1
fi

echo "Latest version: $LATEST_VERSION"

# Construct download URL
TARBALL="macbash-${LATEST_VERSION}-${OS}-${ARCH}.tar.gz"
DOWNLOAD_URL="https://github.com/$REPO/releases/download/v${LATEST_VERSION}/${TARBALL}"

# Create temp directory
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

echo "Downloading $TARBALL..."
curl -fsSL "$DOWNLOAD_URL" -o "$TMPDIR/$TARBALL"

echo "Extracting..."
tar xzf "$TMPDIR/$TARBALL" -C "$TMPDIR"

# Create install directory if needed
if [ ! -d "$INSTALL_DIR" ]; then
    if [ "$USER_INSTALL" = true ]; then
        mkdir -p "$INSTALL_DIR"
    else
        sudo mkdir -p "$INSTALL_DIR"
    fi
fi

# Install binary
echo "Installing to $INSTALL_DIR/macbash..."
if [ "$USER_INSTALL" = true ]; then
    cp "$TMPDIR/macbash-${LATEST_VERSION}-${OS}-${ARCH}/macbash" "$INSTALL_DIR/macbash"
    chmod +x "$INSTALL_DIR/macbash"
else
    sudo cp "$TMPDIR/macbash-${LATEST_VERSION}-${OS}-${ARCH}/macbash" "$INSTALL_DIR/macbash"
    sudo chmod +x "$INSTALL_DIR/macbash"
fi

# Verify installation
if command -v macbash >/dev/null 2>&1; then
    echo ""
    echo "Successfully installed macbash $(macbash --version 2>&1 | head -1)"
else
    echo ""
    echo "macbash installed to $INSTALL_DIR/macbash"

    # Check if install dir is in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            ;;
        *)
            echo ""
            echo "NOTE: $INSTALL_DIR is not in your PATH."
            echo "Add this to your shell profile:"
            echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
            ;;
    esac
fi

echo ""
echo "Usage: macbash [options] <script.sh>"
echo "       macbash --help for more information"
