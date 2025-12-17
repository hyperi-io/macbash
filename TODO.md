# TODO

## Rules to Add

- tar `--exclude` order sensitivity
- AppleDouble `._` file handling
- `cp -R` trailing slash differences
- `mktemp -t` semantic differences
- `/bin/sh` scripts using bashisms

## Enhancements

- Context-aware detection (skip patterns in comments, strings, here-docs)
- AST parsing for complex bash constructs
- More test coverage

## Done

- Project structure and CLI
- YAML rule config
- Rule engine with pattern matching
- Auto-fix for simple transformations (grep -P → grep -E)
- WHY explanations for unfixable patterns
- Console and JSON output
- GitHub Actions CI
- Package distribution (deb, rpm, Homebrew, install script)

## References

- [GNU vs BSD coreutils](https://gist.github.com/skyzyx/3438280b18e4f7c490db8a2a2ca0b9da)
- [sed portability](https://www.johndcook.com/blog/2023/10/18/portable-sed-i/)
- [readlink -f alternatives](https://github.com/ko1nksm/readlinkf)
- [bash 4 features](https://tldp.org/LDP/abs/html/bashver4.html)
