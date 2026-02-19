# TODO

## Enhancements

- AST-based scanning using mvdan.cc/sh/v3 for complex constructs (multi-line patterns, string/variable context)
- Inline comment and quoted string skipping (currently skips full-comment lines and here-docs)

## Done

- Project structure and CLI
- YAML rule config
- Rule engine with pattern matching
- Auto-fix for simple transformations (grep -P → grep -E)
- WHY explanations for unfixable patterns
- Console and JSON output
- GitHub Actions CI
- Package distribution (deb, rpm, Homebrew, install script)
- tar `--exclude` order sensitivity rule
- AppleDouble `._` file handling rule
- `cp -R` trailing slash differences rule
- `mktemp -t` semantic differences rule
- `/bin/sh` bashism detection (shebang-conditional rules via `shebang_match`)
- Fixed `timeout-command` false positive on `--timeout` flags
- Context-aware detection: here-document skipping in scanner
- Shebang-conditional rule support (`shebang_match` field)
- POSIX compliance rules (9 bashism detectors for `/bin/sh` scripts)
- Test coverage for shebang matching, here-doc skipping, timeout false positive

## References

- [GNU vs BSD coreutils](https://gist.github.com/skyzyx/3438280b18e4f7c490db8a2a2ca0b9da)
- [sed portability](https://www.johndcook.com/blog/2023/10/18/portable-sed-i/)
- [readlink -f alternatives](https://github.com/ko1nksm/readlinkf)
- [bash 4 features](https://tldp.org/LDP/abs/html/bashver4.html)
