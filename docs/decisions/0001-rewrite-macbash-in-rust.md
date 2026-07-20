<!--
  Project:   macbash
  File:      docs/decisions/0001-rewrite-macbash-in-rust.md
  Purpose:   Decision record -- port macbash from Go to Rust
  Language:  Markdown

  License:   Apache-2.0
  Copyright: (c) 2025-2026 HYPERI PTY LIMITED
-->

# 0001 -- Rewrite macbash from Go to Rust

**Status:** Accepted. Recorded retrospectively on 2026-07-20; the port itself
shipped in the v1.5.x line.

## Context

macbash began as a Go CLI. HyperI standardised on Rust and Python for new and
maintained work, which left the Go implementation as the only Go artefact in
the estate -- its own toolchain, its own linter config, its own release
plumbing, and no shared CI path with anything else.

The tool is also small and self-contained: a regex corpus, a line scanner, a
fixer and two output formatters. That is about as favourable a rewrite target
as exists.

## Decision

Port the whole tool to Rust 2024 and retire the Go implementation, keeping
byte-for-byte behavioural parity as the acceptance bar rather than
reimplementing from the description.

Parity is enforced, not asserted:

- The rule corpus YAML was carried across unchanged.
- `json_schema_field_names_match_go_oracle` pins the JSON field names against
  the Go struct tags.
- `load_builtin_returns_seventy_three_rules` pins the rule count.
- The CLI flag set, including the hidden deprecated `-f`, matches.

## Consequences

Good:

- One toolchain, one CI path (`hyperi-io/hyperi-ci` rust-ci), one release
  mechanism shared with the rest of the estate.
- A single static binary with no runtime, and a library crate other Rust tools
  can call directly.
- The parity tests double as a permanent regression net.

Costs:

- The `regex` crate is not PCRE. See
  [0003](0003-regex-crate-without-backreferences.md).
- Windows support regressed to EXPERIMENTAL while the cross-build path was
  rebuilt; Go cross-compiled it for free.
- The Go tree lingered untracked on disk long after it stopped being built,
  which is its own lesson: delete the old implementation in the same change
  that lands the new one.
