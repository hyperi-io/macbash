# Project:   machbash-rust
# File:      Makefile
# Purpose:   CI targets wrapping hyperi-ci
#
# License:   Apache-2.0 — HYPERI PTY LIMITED
# Copyright: (c) 2026 HYPERI PTY LIMITED

.PHONY: quality test build check

quality:
	hyperi-ci run quality

test:
	hyperi-ci run test

build:
	hyperi-ci run build

check:
	hyperi-ci check
