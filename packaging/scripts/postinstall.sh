#!/bin/sh
# Project:   macbash
# File:      packaging/scripts/postinstall.sh
# Purpose:   Post-installation script for deb/rpm packages
# Language:  Shell
#
# License:   Apache-2.0
# Copyright: (c) 2025 HyperSec Pty Ltd

set -e

echo "macbash installed successfully!"
echo ""
echo "Usage: macbash [options] <script.sh>"
echo "       macbash --help for more information"
echo ""
echo "To check a script:  macbash script.sh"
echo "To auto-fix:        macbash -w script.sh"
