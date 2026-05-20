# Project:   ${PKG_NAME}
# File:      Formula/${PKG_BINARY}.rb (rendered)
# Purpose:   Homebrew formula
# Language:  Ruby
#
# License:   Apache-2.0
# Copyright: (c) 2025-2026 HYPERI PTY LIMITED

class ${PKG_BREW_CLASS} < Formula
  desc "${PKG_DESCRIPTION}"
  homepage "${PKG_HOMEPAGE}"
  version "${PKG_VERSION}"
  license "${PKG_LICENSE}"

  on_macos do
    if Hardware::CPU.arm?
      url "${PKG_DOWNLOAD_BASE}/v${PKG_VERSION}/${PKG_BINARY}-darwin-arm64.tar.gz"
      sha256 "${PKG_SHA256_DARWIN_ARM64_TARBALL}"
    else
      url "${PKG_DOWNLOAD_BASE}/v${PKG_VERSION}/${PKG_BINARY}-darwin-amd64.tar.gz"
      sha256 "${PKG_SHA256_DARWIN_AMD64_TARBALL}"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "${PKG_DOWNLOAD_BASE}/v${PKG_VERSION}/${PKG_BINARY}-linux-arm64.tar.gz"
      sha256 "${PKG_SHA256_LINUX_ARM64_TARBALL}"
    else
      url "${PKG_DOWNLOAD_BASE}/v${PKG_VERSION}/${PKG_BINARY}-linux-amd64.tar.gz"
      sha256 "${PKG_SHA256_LINUX_AMD64_TARBALL}"
    end
  end

  def install
    bin.install "${PKG_BINARY}"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/${PKG_BINARY} --version")
  end
end
