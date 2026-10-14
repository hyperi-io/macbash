<!--
  Project:   macbash
  File:      docs/rules.md
  Purpose:   Rule schema reference and the false-positive playbook
  Language:  Markdown

  License:   Apache-2.0
  Copyright: (c) 2025-2026 HYPERI PTY LIMITED
-->

# Rules reference

A rule is a regex plus metadata. macbash ships its rules as YAML embedded in
the binary, and `--config <file>` merges more on top. Nothing about a rule is
compiled into Rust, so adding a check is a YAML change.

The schema below is the serde definition in
[src/rules/types.rs](../src/rules/types.rs); the loader validation is in
[src/rules/loader.rs](../src/rules/loader.rs).

## Minimal rule

```yaml
version: "1.0"
rules:
  - id: my-rule
    name: Custom check
    pattern: 'some\s+pattern'
```

`id`, `name` and `pattern` are the only required fields. `severity` defaults
to `warning` and `fix_type` to `suggest`.

Load it with `macbash --config rules.yaml script.sh`.

## Every field

| Field | Type | Default | What it does |
|---|---|---|---|
| `id` | string | required | Unique key. A later rule with the same id REPLACES the earlier one |
| `name` | string | required | One-line title shown in text output |
| `description` | string | `""` | Longer explanation; appears in JSON output |
| `severity` | `error` \| `warning` \| `info` | `warning` | Only `error` affects the exit code |
| `pattern` | regex | required | POSIX ERE, matched against each line |
| `negative_pattern` | regex | `""` | If this matches the line, the rule is suppressed for that line |
| `shebang_match` | regex | `""` | Rule only applies when the file's shebang matches |
| `fix_type` | `suggest` \| `replace` \| `transform` \| `function` | `suggest` | Whether `-w` can rewrite the line |
| `fix_template` | string | `""` | The replacement (for `replace`) or the advice text |
| `why_unfixable` | string | `""` | Shown when a fix cannot be applied automatically |
| `fix_function` | string | `""` | Names a shell helper the fix would need |
| `examples.bad` / `examples.good` | string | `""` | Documentation only |
| `tags` | list | `[]` | Free-form grouping |
| `references` | list | `[]` | URLs backing the rule |
| `test_cases.should_match` | list | `[]` | Lines that MUST fire this rule |
| `test_cases.should_not_match` | list | `[]` | Lines that must NOT fire this rule |

Both `test_cases` lists are mandatory in practice for built-in rules:
`load_builtin_every_rule_has_both_test_case_sides` fails the build if a
shipped rule loses either side.

## Fix types

Only two of the four rewrite anything.

| `fix_type` | `-w` rewrites? | Use it when |
|---|---|---|
| `replace` | yes | The fix is a deterministic regex substitution (`sed -i` -> `sed -i.bak`) |
| `transform` | sometimes | The fix needs analysis of the line -- currently only PCRE-to-ERE for `grep -P` |
| `suggest` | no | The correct fix depends on intent a regex cannot recover |
| `function` | no | The fix needs a helper function pasted into the script |

`suggest` and `function` findings count as unfixable, so `macbash -w` can exit
1 having written nothing. That is not a failure -- it means the remaining
issues need a human.

### Writing a replace template

The template is a `regex` crate REPLACEMENT string, not a plain literal. Two
consequences bite hard:

- **`$` starts a capture reference.** To emit a literal `$` you write `$$`.
  Brace the capture (`${1}`, not `$1`) whenever the next character could be
  read as part of a group name.
- **Whatever the pattern matches is what gets replaced.** If the pattern
  consumes a character to prove a match -- the `\w` after `|&`, the trailing
  `\s` after `sed -r` -- the template has to put it back, or the rewrite eats
  it.

Both mistakes produce a rewrite that still parses as bash, so `bash -n`
happily lets it through. `every_rewriting_rule_turns_its_bad_example_into_its_good_example`
in [src/fixer.rs](../src/fixer.rs) is the guard: it runs every `replace` and
`transform` rule's `examples.bad` through the real fixer and demands exactly
`examples.good`. Keep both examples honest and it cannot regress.

### Only auto-fix where a portable form exists

`fix_type: replace` is a promise that the rewrite is safe on BOTH platforms.
Some GNU-isms have no portable spelling -- `date -d @EPOCH` (GNU) versus
`date -r EPOCH` (BSD, where GNU's `-r` means `--reference=FILE`), or `xargs -r`
(GNU-only, and dropping it changes behaviour on GNU because GNU xargs runs the
command once on empty input). Auto-fixing those turns a working Linux script
into a macOS-only one, silently.

Those rules stay `fix_type: suggest` with a `why_unfixable` that names the
trade-off. Reporting a problem you cannot safely fix is the correct outcome.

## Writing a pattern that does not cry wolf

The `regex` crate has **no backreferences and no lookaround**. You cannot
write `(?!...)` or `\1`. When a pattern needs "match X except when Y", you have
two levers, and the order matters.

**First, try to fold the exclusion into the pattern.** That keeps the rule
per-match. The worked example is `sed-inplace-no-backup` in
[src/rules/builtin/coreutils.yaml](../src/rules/builtin/coreutils.yaml). BSD
sed needs an argument after `-i`, so `sed -i 's/a/b/' f` silently eats the
script as the backup suffix -- a real bug worth an error. But `sed -i '' ...`
is the CORRECT BSD form. The natural way to tell them apart is "the same quote
twice", `(['"])\1`, which needs a backreference. Requiring a NON-quote after
the opening quote says the same thing without one:

```yaml
pattern: '\bsed\s+(-[a-zA-Z]*i)\s+([''"][^''"])'
```

**Only then reach for `negative_pattern`,** and know what it costs:

> **Note:** `negative_pattern` vetoes the whole LINE for that rule, not the
> individual match. `sed -i '' 's/a/b/' f1 && sed -i 's/c/d/' f2` would report
> nothing at all -- and so would a line where an unrelated quoted string
> merely mentions the excluded form.

Use it when the exclusion genuinely depends on context outside the match, as
`bash4-pipe-stderr` does: whether a `|&` sits inside a string literal depends
on quotes arbitrarily far to its left. Full reasoning in
[decisions/0003-regex-crate-without-backreferences.md](decisions/0003-regex-crate-without-backreferences.md).

## Gating a rule to /bin/sh

`shebang_match` is the second lever. POSIX-only complaints must not fire on a
bash script:

```yaml
shebang_match: '^#!/bin/sh'
```

The scanner compares this against line 1 of the file. A rule with no
`shebang_match` applies everywhere.

## Overriding a built-in rule

Reuse the id. The merge is last-wins, so a `--config` file can retune a
built-in rule for your codebase without forking the corpus. Raising
`shebang-bin-bash` from `info` to `error` makes an `#!/bin/bash` shebang fail
CI:

```yaml
version: "1.0"
rules:
  - id: shebang-bin-bash
    name: "#!/bin/bash shebang"
    pattern: '^#!/bin/bash\s*$'
    severity: error         # built-in ships this as info
    fix_type: replace
    fix_template: "#!/usr/bin/env bash"
```

Verify the override took effect on a script whose only issue is its shebang:

```console
$ macbash plain.sh; echo "exit=$?"
No issues found.
exit=0

$ macbash --config soften.yaml plain.sh; echo "exit=$?"
...
Found 1 error(s) in 1 file(s)
exit=1
```

The severity change alone flips the exit code, which is what makes an
override useful in CI.

## Adding a built-in rule

1. Pick the right pack: `bash.yaml` (bash 4+ features), `coreutils.yaml` (GNU
   flags), `posix.yaml` (portability and shebang concerns).
2. Write the rule with at least one `should_match` and one `should_not_match`
   case. Include the near-miss that would false-positive.
3. Run `cargo test`. Two corpus tests replay every example through the real
   scanner, so a wrong pattern fails immediately.
4. Update `load_builtin_returns_seventy_three_rules` in
   [src/rules/loader.rs](../src/rules/loader.rs). That count is a deliberate
   parity guard against the Go binary macbash replaced -- changing it is a
   decision, not a chore.

## AI steering

Rule authoring is where AI assistance goes wrong most reliably in this
codebase, because a plausible-looking regex passes review and fails on real
scripts.

| Don't | Do | Why |
|---|---|---|
| Write `(?!...)` or `\1` in a pattern | Use `negative_pattern` | The `regex` crate rejects both at compile time; the rule pack then fails to load at all |
| Add a rule with only `should_match` cases | Add the near-miss that would false-positive | A rule with no negative cases is how issue #6 shipped |
| Assume a `replace` template is a plain literal | Escape a literal `$` as `$$`, brace captures as `${1}` | It is a regex replacement string; the obvious spelling silently expands to nothing |
| Let the pattern consume a character the template does not restore | Capture it and put it back | `\|&\s*\w` with template `2>&1 \|` rewrote `\|& grep` to `\|rep` -- valid bash, wrong command |
| Bump the rule-count test to make `cargo test` green | Decide whether the new rule belongs, then bump it | The count is a parity guard, not a counter |
| Set `severity: error` by default | Reserve `error` for "this is broken on macOS" | Only `error` fails CI for consumers |
| Give a `suggest` rule a `fix_template` that looks like a replacement | Write advice text, and fill `why_unfixable` | `-w` will not apply it, and a replacement-shaped string implies it will |
| Auto-fix a GNU-ism that has no portable equivalent | Leave it `suggest` and explain in `why_unfixable` | `-w` would silently convert a working Linux script into a macOS-only one |
| Reach for `negative_pattern` first | Fold the exclusion into the pattern if you can | The veto kills the whole line, including a real bug sharing it |

Verify any rule change with:

```sh
cargo test
cargo run -- --severity info tests/fixtures/deploy-script.sh
```
