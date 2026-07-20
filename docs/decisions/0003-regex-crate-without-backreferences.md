<!--
  Project:   macbash
  File:      docs/decisions/0003-regex-crate-without-backreferences.md
  Purpose:   Decision record -- accept the regex crate's missing backreferences
             and lookaround, and prefer a tighter pattern over a line veto
  Language:  Markdown

  License:   Apache-2.0
  Copyright: (c) 2025-2026 HYPERI PTY LIMITED
-->

# 0003 -- Accept the `regex` crate without backreferences or lookaround

**Status:** Accepted. Recorded 2026-07-20, prompted by issue #6.

## Context

Rust's `regex` crate guarantees linear-time matching, and pays for it by
refusing to implement backreferences (`\1`) and lookaround (`(?=)`, `(?!)`).
Go's `regexp` has the same restriction, so the port inherited the constraint
rather than introducing it.

Some portability rules genuinely need "match X except when Y". The worked case
is `sed-inplace-no-backup`. BSD sed requires an argument after `-i`, so
`sed -i 's/a/b/' f` silently consumes the script as the backup suffix -- a
real bug. But `sed -i '' 's/a/b/' f` is the CORRECT BSD form, and the two look
alike to a pattern that can only say "`-i`, whitespace, a quote".

The natural expression is a backreference -- `-i\s+(['"])\1` for "the same
quote twice" -- which the engine rejects at compile time. Issue #6 was exactly
this: the correct macOS form was flagged as an ERROR, failing CI for scripts
that were right.

## Considered options

- **Swap to `fancy-regex`** (supports backreferences and lookaround). Rejected:
  it reintroduces backtracking and therefore catastrophic-blowup risk on
  attacker-supplied or merely unlucky input, on a tool people run over whole
  repositories in CI. The linear-time guarantee is load-bearing because
  `--config` lets a user supply arbitrary patterns.
- **Post-filter matches in Rust.** Rejected: it moves rule semantics out of the
  YAML and into code, which contradicts
  [0002](0002-embed-the-rule-corpus-as-yaml.md) and makes rules unauditable
  from the corpus alone.
- **Veto the line with `negative_pattern`.** Rejected -- see below. This was
  the first fix attempted for issue #6, and reviewing it is what produced this
  record.
- **Tighten the main pattern so the exception never matches.** Chosen.

## Decision

Keep the `regex` crate. Where an exclusion can be folded into the main pattern,
fold it in -- that keeps the rule per-MATCH. For
`sed-inplace-no-backup`, requiring a non-quote character straight after the
opening quote is exactly "the quote is not immediately closed":

```yaml
pattern: '\bsed\s+(-[a-zA-Z]*i)\s+([''"][^''"])'
```

Reach for `negative_pattern` only when the exclusion genuinely cannot be
expressed in the match itself -- for example `bash4-pipe-stderr`, where
"is this `|&` inside a string literal?" depends on quotes arbitrarily far to
the left of the match.

## Why not `negative_pattern` here

**`negative_pattern` vetoes the whole LINE for that rule, not the individual
match.** For the `sed` case that is far more expensive than it looks:

```sh
sed -i '' 's/a/b/' f1 && sed -i 's/c/d/' f2   # second sed is a real bug
echo "backup with sed -i '' style" ; sed -i 's/real/bug/' prod.conf
```

Both lines contain a genuine, ERROR-severity defect, and a line veto reports
neither. The second is the worse shape: an unrelated mention inside a quoted
string suppresses a real finding.

Measured on a five-line file of realistic combinations, the veto reported 1 of
5 genuine errors; the tightened pattern reports all 5, with every one of the
rule's `should_not_match` cases still clean.

## Consequences

Good:

- Linear-time matching holds for every rule, including user-supplied ones from
  `--config`. A hostile custom rule cannot hang the scanner.
- Exclusions live next to the pattern they modify, so a reviewer sees the whole
  rule in one place.
- Per-match exclusion means one bad usage on a busy line is still reported.

Costs:

- Alternations and character classes must be written out by hand, so a new
  quoting or spelling variant is a rule edit rather than something the pattern
  generalises over. New variants therefore need a `should_not_match` case.
- Where `negative_pattern` IS the only option, the line-veto false negative is
  real and accepted. `bash4-pipe-stderr` deliberately takes it: suppressing a
  `|&` that follows an opening quote costs a missed finding when a line has
  both a string and a real operator, which is much cheaper than rewriting the
  inside of a string literal.
- A few residual gaps remain in the `sed` pattern -- `sed -i "'s/a/b/'"`
  (script opening with the other quote kind) and a bare `sed -i '` at end of
  line are not matched. Both are rarer than what the veto cost.
