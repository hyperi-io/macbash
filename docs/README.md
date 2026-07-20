<!--
  Project:   macbash
  File:      docs/README.md
  Purpose:   Landing page for the macbash documentation set
  Language:  Markdown

  License:   Apache-2.0
  Copyright: (c) 2025-2026 HYPERI PTY LIMITED
-->

# macbash documentation

Install and usage live in the [README](../README.md). This directory holds the
depth.

| Doc | Read it when |
|---|---|
| [../ARCHITECTURE.md](../ARCHITECTURE.md) | You are changing the code and need the map |
| [rules.md](rules.md) | You are writing, overriding or debugging a rule |
| [release.md](release.md) | You want to know how a commit becomes a published binary |
| [decisions/](decisions/) | You are about to re-litigate a design choice |
| [../packaging/README.md](../packaging/README.md) | You want the packaging framework for another Rust CLI |

## Quick orientation

macbash reads bash scripts line by line and matches them against a corpus of
regex rules that encode where GNU/Linux and macOS BSD userland diverge. It
reports findings by default and can rewrite a subset of them in place.

Three things are worth knowing before you change anything:

- Rules are YAML, not Rust. Adding a check means editing
  `src/rules/builtin/*.yaml` -- see [rules.md](rules.md).
- The JSON output schema is a stable contract, pinned by a test against the Go
  implementation macbash replaced. Text output is not.
- The regex engine has no backreferences and no lookaround. Exclusions go in
  `negative_pattern` -- see
  [decisions/0003-regex-crate-without-backreferences.md](decisions/0003-regex-crate-without-backreferences.md).
