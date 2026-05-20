#!/bin/sh
#  Project:   rust-cli-packaging (in-tree framework)
#  File:      packaging/render-release.sh
#  Purpose:   Render per-version templates (homebrew, nfpm, winget, scoop)
#             using checksums fetched from R2 (default) or a local dir.
#  Language:  Shell (POSIX)
#
#  License:   Apache-2.0
#  Copyright: (c) 2025-2026 HYPERI PTY LIMITED
#
#  Usage:
#    packaging/render-release.sh <version>                 # fetch sha256s from R2
#    packaging/render-release.sh <version> --from-dir <d>  # use local checksums
#    NFPM_ARCH=amd64 packaging/render-release.sh <version> # render nfpm.yaml for a specific arch
#
#  Expected R2 layout (set up by cross-build.yml):
#    ${PKG_DOWNLOAD_BASE}/v${VERSION}/${PKG_BINARY}-${os}-${arch}.sha256
#      contains two lines:
#        <sha>  ${PKG_BINARY}-${os}-${arch}         (raw binary)
#        <sha>  ${PKG_BINARY}-${os}-${arch}.tar.gz  (archive, or .zip on windows)

set -eu

if [ $# -lt 1 ]; then
    echo "usage: render-release.sh <version> [--from-r2 | --from-dir <dir>]" >&2
    exit 1
fi

VERSION="$1"; shift
SOURCE="r2"
DIR=""

while [ $# -gt 0 ]; do
    case "$1" in
        --from-r2)  SOURCE="r2" ;;
        --from-dir) shift; SOURCE="dir"; DIR="${1:?--from-dir needs a path}" ;;
        *) echo "unknown arg: $1" >&2; exit 1 ;;
    esac
    shift
done

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
PROJECT_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
CONFIG="${PACKAGING_ENV:-$PROJECT_ROOT/packaging.env}"
TEMPLATES="$SCRIPT_DIR/templates-release"
DIST="$SCRIPT_DIR/dist/release"

[ -f "$CONFIG" ] || { echo "render-release.sh: $CONFIG not found" >&2; exit 1; }
command -v envsubst >/dev/null 2>&1 || { echo "render-release.sh: envsubst not found (install gettext)" >&2; exit 1; }

# shellcheck disable=SC1090
. "$CONFIG"

: "${PKG_NAME:?packaging.env must set PKG_NAME}"
: "${PKG_BINARY:?packaging.env must set PKG_BINARY}"
: "${PKG_DOWNLOAD_BASE:?packaging.env must set PKG_DOWNLOAD_BASE}"
: "${PKG_DESCRIPTION:?packaging.env must set PKG_DESCRIPTION}"
: "${PKG_HOMEPAGE:?packaging.env must set PKG_HOMEPAGE}"
: "${PKG_LICENSE:?packaging.env must set PKG_LICENSE}"
: "${PKG_MAINTAINER:?packaging.env must set PKG_MAINTAINER}"
: "${PKG_VENDOR:?packaging.env must set PKG_VENDOR}"
: "${PKG_GITHUB_REPO:?packaging.env must set PKG_GITHUB_REPO}"
: "${PKG_BREW_CLASS:?packaging.env must set PKG_BREW_CLASS}"
: "${PKG_WINGET_ID:?packaging.env must set PKG_WINGET_ID}"
: "${PKG_WINGET_PUBLISHER:?packaging.env must set PKG_WINGET_PUBLISHER}"

PKG_VERSION="$VERSION"
TAG="v$VERSION"
NFPM_ARCH="${NFPM_ARCH:-amd64}"

# Architectures we may have checksums for. Missing ones get empty SHA values
# (templates that reference them will be malformed if rendered without the
# matching arch — that's intentional; the absence is loud).
ARCHES="linux-amd64 linux-arm64 darwin-amd64 darwin-arm64 windows-amd64"

WORK=$(mktemp -d "${TMPDIR:-/tmp}/render-release.XXXXXX")
trap 'rm -rf "$WORK"' EXIT

fetch_sums() {
    os_arch="$1"
    out="$WORK/${os_arch}.sha256"
    if [ "$SOURCE" = "r2" ]; then
        url="$PKG_DOWNLOAD_BASE/$TAG/${PKG_BINARY}-${os_arch}.sha256"
        if curl -fsSL "$url" -o "$out" 2>/dev/null; then
            return 0
        fi
        return 1
    fi
    src="$DIR/${PKG_BINARY}-${os_arch}.sha256"
    if [ -f "$src" ]; then
        cp "$src" "$out"
        return 0
    fi
    return 1
}

extract_sum() {
    file="$1"
    pattern="$2"
    grep -E "  ${pattern}\$" "$file" 2>/dev/null | awk '{print $1}' | head -1
}

# Populate per-arch SHA exports
for os_arch in $ARCHES; do
    upper=$(echo "$os_arch" | tr '[:lower:]-' '[:upper:]_')
    bin_var="PKG_SHA256_${upper}_BINARY"
    tar_var="PKG_SHA256_${upper}_TARBALL"
    zip_var="PKG_SHA256_${upper}_ZIP"

    if fetch_sums "$os_arch"; then
        case "$os_arch" in
            windows-*)
                bin_match="${PKG_BINARY}-${os_arch}.exe"
                archive_match="${PKG_BINARY}-${os_arch}.zip"
                archive_var="$zip_var"
                ;;
            *)
                bin_match="${PKG_BINARY}-${os_arch}"
                archive_match="${PKG_BINARY}-${os_arch}.tar.gz"
                archive_var="$tar_var"
                ;;
        esac
        bin_sum=$(extract_sum "$WORK/${os_arch}.sha256" "$bin_match" || echo "")
        archive_sum=$(extract_sum "$WORK/${os_arch}.sha256" "$archive_match" || echo "")
        eval "$bin_var=\"\$bin_sum\""
        eval "$archive_var=\"\$archive_sum\""
        eval "export $bin_var $archive_var"
        echo "  $os_arch: bin=${bin_sum:-?}  archive=${archive_sum:-?}"
    else
        echo "  $os_arch: (no checksums available)" >&2
    fi
done

export PKG_NAME PKG_BINARY PKG_VERSION PKG_DOWNLOAD_BASE PKG_DESCRIPTION \
    PKG_HOMEPAGE PKG_LICENSE PKG_MAINTAINER PKG_VENDOR PKG_GITHUB_REPO \
    PKG_BREW_CLASS PKG_WINGET_ID PKG_WINGET_PUBLISHER NFPM_ARCH

WHITELIST='${PKG_NAME} ${PKG_BINARY} ${PKG_VERSION} ${PKG_DOWNLOAD_BASE}
${PKG_DESCRIPTION} ${PKG_HOMEPAGE} ${PKG_LICENSE} ${PKG_MAINTAINER}
${PKG_VENDOR} ${PKG_GITHUB_REPO} ${PKG_BREW_CLASS} ${PKG_WINGET_ID}
${PKG_WINGET_PUBLISHER} ${NFPM_ARCH}
${PKG_SHA256_LINUX_AMD64_BINARY} ${PKG_SHA256_LINUX_AMD64_TARBALL}
${PKG_SHA256_LINUX_ARM64_BINARY} ${PKG_SHA256_LINUX_ARM64_TARBALL}
${PKG_SHA256_DARWIN_AMD64_BINARY} ${PKG_SHA256_DARWIN_AMD64_TARBALL}
${PKG_SHA256_DARWIN_ARM64_BINARY} ${PKG_SHA256_DARWIN_ARM64_TARBALL}
${PKG_SHA256_WINDOWS_AMD64_BINARY} ${PKG_SHA256_WINDOWS_AMD64_ZIP}'

rm -rf "$DIST"
mkdir -p "$DIST"

count=0
# Walk every file under templates-release/ and mirror the relative path
# into dist/release/, swapping `formula.rb` -> `${PKG_BINARY}.rb` and
# winget `version.yaml` -> `${PKG_WINGET_ID}.yaml` etc.
find "$TEMPLATES" -type f | while IFS= read -r tmpl; do
    rel=${tmpl#"$TEMPLATES/"}
    # Friendly output names per channel
    case "$rel" in
        homebrew/formula.rb)
            out="homebrew/${PKG_BINARY}.rb" ;;
        winget/version.yaml)
            out="winget/${PKG_WINGET_ID}.yaml" ;;
        winget/installer.yaml)
            out="winget/${PKG_WINGET_ID}.installer.yaml" ;;
        winget/locale.en-US.yaml)
            out="winget/${PKG_WINGET_ID}.locale.en-US.yaml" ;;
        scoop/manifest.json)
            out="scoop/${PKG_BINARY}.json" ;;
        *) out="$rel" ;;
    esac
    mkdir -p "$DIST/$(dirname "$out")"
    envsubst "$WHITELIST" < "$tmpl" > "$DIST/$out"
    count=$((count + 1))
done

echo "rendered release templates -> $DIST (version $PKG_VERSION, source $SOURCE)"
find "$DIST" -type f | sort
