# 1.0.0 (2025-12-17)


* feat!: v1.0 release - auto-fix, packaging, Apache-2.0 ([6e2b6df](https://github.com/hypersec-io/macbash/commit/6e2b6dff4802241d7922993b70614fe8178beb46))


### Bug Fixes

* add missing cmd/macbash/main.go to repo ([9b11f12](https://github.com/hypersec-io/macbash/commit/9b11f126862e64eee2e9a11c17daa34255d506ea))
* gosec exclusions and nfpm envsubst for path expansion ([c8a76f2](https://github.com/hypersec-io/macbash/commit/c8a76f25c8144686f4464ee692dd99c36ad68e98))
* remove gitleaks action (requires org license) ([e4714f3](https://github.com/hypersec-io/macbash/commit/e4714f38f7393f9a62d3e220d6ae74a9bccd0151))
* resolve golangci-lint errors and nfpm config ([c815f62](https://github.com/hypersec-io/macbash/commit/c815f62ccedce2c01d8422b337be43dd68d6b482))
* update nfpm version to 2.44.0 ([08cabfd](https://github.com/hypersec-io/macbash/commit/08cabfdbba154ce04c717c812717c6fe74b1e222))
* use dynamic versions for dependencies ([ba3e469](https://github.com/hypersec-io/macbash/commit/ba3e46907fad836adf068e2a3cebdb9978766e5f))
* use repository variable for gitleaks toggle ([e95c1f4](https://github.com/hypersec-io/macbash/commit/e95c1f4fb51ad42cc4e167a828e4e0bb0bb7c741))


### Features

* add gitleaks with optional license key ([39901eb](https://github.com/hypersec-io/macbash/commit/39901eb36ef941d650801c28cf2e9dc017ba14de))
* add output path options for fixed files ([9d2d460](https://github.com/hypersec-io/macbash/commit/9d2d4604c80c4a5367aafe238bea8679354b9423))
* initial macbash implementation ([b037fc7](https://github.com/hypersec-io/macbash/commit/b037fc79421902d33284d2d2819ae416391586aa))


### BREAKING CHANGES

* CLI flags changed from --fix to -w/--write and -o/--output

Features:
- Auto-fix with PCRE to ERE transformation for grep -P
- WHY explanations for unfixable patterns
- Bash syntax validation before writing fixes
- Package distribution: deb, rpm, Homebrew, install script

Changes:
- Apache-2.0 licensing (was unlicensed)
- Human-style code and docs (removed LLM patterns)
- Simplified TODO to remaining work only
