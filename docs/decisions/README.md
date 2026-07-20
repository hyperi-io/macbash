<!--
  Project:   macbash
  File:      docs/decisions/README.md
  Purpose:   Index of architecture decision records
  Language:  Markdown

  License:   Apache-2.0
  Copyright: (c) 2025-2026 HYPERI PTY LIMITED
-->

# Decision records

One immutable file per decision, MADR format. An accepted record is never
edited -- if the decision changes, add a new record and mark the old one
superseded.

| # | Decision | Status |
|---|---|---|
| [0001](0001-rewrite-macbash-in-rust.md) | Rewrite macbash from Go to Rust | Accepted |
| [0002](0002-embed-the-rule-corpus-as-yaml.md) | Ship the rule corpus as YAML embedded in the binary | Accepted |
| [0003](0003-regex-crate-without-backreferences.md) | Accept the `regex` crate despite no backreferences or lookaround | Accepted |
