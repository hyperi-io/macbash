<!--
  Project:   macbash
  File:      docs/decisions/0002-embed-the-rule-corpus-as-yaml.md
  Purpose:   Decision record -- rules live in YAML, compiled into the binary
  Language:  Markdown

  License:   Apache-2.0
  Copyright: (c) 2025-2026 HYPERI PTY LIMITED
-->

# 0002 -- Ship the rule corpus as YAML embedded in the binary

**Status:** Accepted. Carried over from the Go implementation; recorded
2026-07-20.

## Context

macbash needs a large, growing set of checks. Three shapes were available:

- **Rules as Rust code.** Fastest, type-checked, but every new check is a code
  change, a review and a release, and users cannot extend it.
- **Rules as YAML loaded from disk at runtime.** Extensible, but the binary
  stops being self-contained: it needs an install prefix, a search path, and a
  story for what happens when the file is missing or stale.
- **Rules as YAML embedded at compile time.** Self-contained binary, plus an
  identical schema available to users through `--config`.

## Decision

Author the corpus as YAML under `src/rules/builtin/` and embed it with
`rust-embed`. Users extend or override it with `--config <file>`, which parses
through the SAME schema and merges last-wins by rule id.

## Consequences

Good:

- The binary is genuinely standalone -- `curl | sh` installs one file that
  works.
- Adding a check is a YAML pull request. No Rust knowledge required, and the
  diff is readable by anyone who understands the shell problem.
- Because the built-in and custom schemas are identical, every field a user
  can set is one we already exercise; there is no second-class extension API.
- Overriding by id means a consumer can retune a noisy rule without forking.

Costs:

- Rule changes still need a release to reach users, since the corpus is baked
  in. `--config` is the escape hatch in the meantime.
- YAML gives no compile-time safety, so validation is runtime: the loader
  rejects a missing id, name or pattern and compiles every regex up front,
  and two corpus-wide tests replay every `test_cases` example through the real
  scanner.
- Rules are regexes, so they inherit the engine's limits -- see
  [0003](0003-regex-crate-without-backreferences.md).
