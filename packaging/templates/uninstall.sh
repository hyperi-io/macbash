#!/bin/sh
#  Project:   ${PKG_NAME}
#  File:      uninstall.sh (rendered)
#  Purpose:   POSIX uninstaller — locates and removes ${PKG_BINARY}
#  Language:  Shell (POSIX)
#
#  License:   Apache-2.0
#  Copyright: (c) 2025-2026 HYPERI PTY LIMITED
#
#  Usage:
#    curl -fsSL ${PKG_DOWNLOAD_BASE}/uninstall.sh | sh
#    curl -fsSL ${PKG_DOWNLOAD_BASE}/uninstall.sh | sh -s -- --user
#    curl -fsSL ${PKG_DOWNLOAD_BASE}/uninstall.sh | sh -s -- --dir /opt/bin
#    curl -fsSL ${PKG_DOWNLOAD_BASE}/uninstall.sh | sh -s -- --all     # remove every copy on PATH

set -eu

BINARY="${PKG_BINARY}"
DEFAULT_DIR="${PKG_DEFAULT_DIR}"
USER_DIR="${PKG_USER_DIR}"

MODE="auto"
TARGET_DIR=""

while [ $# -gt 0 ]; do
    case "$1" in
        --user)    MODE="user" ;;
        --system)  MODE="system" ;;
        --dir)     shift; MODE="dir"; TARGET_DIR="$1" ;;
        --all)     MODE="all" ;;
        --help)
            cat <<EOF
Usage: uninstall.sh [options]
  --user           Remove from $USER_DIR
  --system         Remove from $DEFAULT_DIR
  --dir <path>     Remove from a specific directory
  --all            Remove every $BINARY found on PATH and common install dirs
  (default)        auto-detect using command -v
  --help           Show this help
EOF
            exit 0
            ;;
        *) echo "uninstall.sh: unknown option: $1" >&2; exit 1 ;;
    esac
    shift
done

# Candidate locations to check in --all mode + first-hit fallback in auto mode.
# Includes the configured user/system dirs plus typical Rust/Linux paths.
candidates() {
    echo "$DEFAULT_DIR/$BINARY"
    echo "$USER_DIR/$BINARY"
    echo "/usr/local/bin/$BINARY"
    echo "/usr/bin/$BINARY"
    echo "/opt/homebrew/bin/$BINARY"
    echo "/home/linuxbrew/.linuxbrew/bin/$BINARY"
}

remove_one() {
    path="$1"
    if [ ! -e "$path" ] && [ ! -L "$path" ]; then
        return 1
    fi
    if [ -w "$path" ] || [ -w "$(dirname "$path")" ]; then
        rm -f "$path"
    else
        echo "Removing $path (requires sudo)..."
        sudo rm -f "$path"
    fi
    echo "Removed $path"
    return 0
}

case "$MODE" in
    auto)
        found=$(command -v "$BINARY" 2>/dev/null || true)
        if [ -z "$found" ]; then
            echo "$BINARY: not found on PATH. Use --all to scan common install dirs."
            exit 0
        fi
        remove_one "$found" || true
        ;;
    user)
        remove_one "$USER_DIR/$BINARY" || echo "$BINARY: not present in $USER_DIR"
        ;;
    system)
        remove_one "$DEFAULT_DIR/$BINARY" || echo "$BINARY: not present in $DEFAULT_DIR"
        ;;
    dir)
        remove_one "$TARGET_DIR/$BINARY" || echo "$BINARY: not present in $TARGET_DIR"
        ;;
    all)
        any=0
        # First, drain everything from PATH. Use `tr` to split on ':' rather
        # than mutating IFS globally; semgrep ifs-tampering.
        while IFS= read -r d; do
            [ -n "$d" ] || continue
            if [ -e "$d/$BINARY" ] || [ -L "$d/$BINARY" ]; then
                remove_one "$d/$BINARY" && any=1
            fi
        done <<EOF
$(printf '%s\n' "$PATH" | tr ':' '\n')
EOF
        # Then sweep the candidate dirs that may not be on PATH.
        # Use `for` (no pipe) so $any survives — pipe creates a subshell.
        for path in $(candidates); do
            if [ -e "$path" ] || [ -L "$path" ]; then
                remove_one "$path" && any=1
            fi
        done
        if [ "$any" = 0 ]; then
            echo "$BINARY: no installations found."
        fi
        ;;
esac

# Final check
if command -v "$BINARY" >/dev/null 2>&1; then
    leftover=$(command -v "$BINARY")
    echo ""
    echo "Note: $BINARY is still on PATH at $leftover."
    echo "  Re-run with --all to remove every copy, or --dir $(dirname "$leftover") to target it."
fi
