# Project:   macbash
# File:      packaging/homebrew/macbash.rb
# Purpose:   Homebrew formula for macbash
# Language:  Ruby
#
# License:   Apache-2.0
# Copyright: (c) 2025 HyperSec Pty Ltd

class Macbash < Formula
  desc "Bash script compatibility checker for macOS"
  homepage "https://github.com/hypersec-io/macbash"
  version "${VERSION}"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/hypersec-io/macbash/releases/download/v${VERSION}/macbash-darwin-arm64.tar.gz"
      sha256 "${SHA256_DARWIN_ARM64}"
    else
      url "https://github.com/hypersec-io/macbash/releases/download/v${VERSION}/macbash-darwin-amd64.tar.gz"
      sha256 "${SHA256_DARWIN_AMD64}"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/hypersec-io/macbash/releases/download/v${VERSION}/macbash-linux-arm64.tar.gz"
      sha256 "${SHA256_LINUX_ARM64}"
    else
      url "https://github.com/hypersec-io/macbash/releases/download/v${VERSION}/macbash-linux-amd64.tar.gz"
      sha256 "${SHA256_LINUX_AMD64}"
    end
  end

  def install
    bin.install "macbash"
  end

  test do
    # Test that macbash runs and shows version
    assert_match version.to_s, shell_output("#{bin}/macbash --version")

    # Test basic functionality - create a test script with a known issue
    (testpath/"test.sh").write <<~EOS
      #!/bin/bash
      grep -P '\\d+' file.txt
    EOS

    output = shell_output("#{bin}/macbash #{testpath}/test.sh 2>&1", 1)
    assert_match "grep-perl-regex", output
  end
end
