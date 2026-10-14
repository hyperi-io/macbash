# Changelog

Rendered by CI and committed back at the end of a release -- do not edit by
hand. Release notes also appear on the GitHub Releases page, one per tag.

## [1.5.12](https://github.com/hyperi-io/macbash/compare/v1.5.11...v1.5.12) (2026-10-14)

### Bug Fixes

* key the linux tarball upload on the tarball ([#17](https://github.com/hyperi-io/macbash/issues/17)) ([7ee41d7](https://github.com/hyperi-io/macbash/commit/7ee41d7ef4fccfafcc69e2f87d78cf6f58f0108e))
* make install.ps1 fail closed on checksums ([#18](https://github.com/hyperi-io/macbash/issues/18)) ([1b4501b](https://github.com/hyperi-io/macbash/commit/1b4501bb8cfd76b86f88cea8bc90ed81d03ecf7b))
* refuse unverified installs and fork publishes ([#16](https://github.com/hyperi-io/macbash/issues/16)) ([79f68a8](https://github.com/hyperi-io/macbash/commit/79f68a8c6bfa5c7c29ad2397a608c0bd3ee5ecb6)), closes [#10](https://github.com/hyperi-io/macbash/issues/10)

## [1.5.11](https://github.com/hyperi-io/macbash/compare/v1.5.10...v1.5.11) (2026-10-14)

### Bug Fixes

* **ci:** accept from-head and bump on dispatch, like the rest of the fleet ([35e8c58](https://github.com/hyperi-io/macbash/commit/35e8c58f817ad5755a7bd59b22d3ddd6b2276ff8)), closes [hyperi-ci#88](https://github.com/hyperi-io/hyperi-ci/issues/88)
* **ci:** mint the App token from the client ID, not the app ID ([#14](https://github.com/hyperi-io/macbash/issues/14)) ([2dca91a](https://github.com/hyperi-io/macbash/commit/2dca91a4c5d18b0606f3a7256d1bf6d6e054f0be)), closes [hyperi-io/hyperi-ci#100](https://github.com/hyperi-io/hyperi-ci/issues/100)
* clear the sweep warnings and move YAML parsing to serde-saphyr ([#15](https://github.com/hyperi-io/macbash/issues/15)) ([3d39ab9](https://github.com/hyperi-io/macbash/commit/3d39ab9d6e7efc4d1812657d5fa4d31a609fa255))

## [1.5.9](https://github.com/hyperi-io/macbash/compare/v1.5.8...v1.5.9) (2026-05-27)


### Bug Fixes

* **ci:** silence semgrep shell-injection + ifs-tampering on quality gate ([98726ed](https://github.com/hyperi-io/macbash/commit/98726ed109580a412934dc201a199a2f5f31a86a)), closes [#5](https://github.com/hyperi-io/macbash/issues/5)
* **deps:** drop vergen-gix; fixes 7 GHSA in gix family ([4319f45](https://github.com/hyperi-io/macbash/commit/4319f4574cb3cba25afa1193d672319870735eed))

## [1.5.8](https://github.com/hyperi-io/macbash/compare/v1.5.7...v1.5.8) (2026-05-21)


### Bug Fixes

* bundle README into v1.5.8 release ([0075ccc](https://github.com/hyperi-io/macbash/commit/0075cccef1e52c2520f8c7dfe9ec73fa14bd0d21))

## [1.5.7](https://github.com/hyperi-io/macbash/compare/v1.5.6...v1.5.7) (2026-05-21)


### Bug Fixes

* package.yml resolves tag for workflow_run trigger ([e06e5c4](https://github.com/hyperi-io/macbash/commit/e06e5c4f17534d300e71441503ef642cb8c1c66e))

## [1.5.6](https://github.com/hyperi-io/macbash/compare/v1.5.5...v1.5.6) (2026-05-21)


### Bug Fixes

* retrigger CI for workflow_run chain wiring ([b5c0e34](https://github.com/hyperi-io/macbash/commit/b5c0e34b582dd9a41e84e72d39cc9fda047ac5e9))

## [1.5.5](https://github.com/hyperi-io/macbash/compare/v1.5.4...v1.5.5) (2026-05-21)


### Bug Fixes

* package.yml now uploads static install scripts to R2 ([1ded08d](https://github.com/hyperi-io/macbash/commit/1ded08d073de4d1113adee8fd28204f3ef3dda98))

## [1.5.4](https://github.com/hyperi-io/macbash/compare/v1.5.3...v1.5.4) (2026-05-21)


### Bug Fixes

* keep Cargo.lock in sync with Cargo.toml on every release ([e457bf5](https://github.com/hyperi-io/macbash/commit/e457bf596781724b789879914f8e4a19b42897bf))
* retrigger release for cargo.lock sync ([6c3a1e9](https://github.com/hyperi-io/macbash/commit/6c3a1e91805c5fb318f7e901da781f23c658cd0d))

## [1.5.3](https://github.com/hyperi-io/macbash/compare/v1.5.2...v1.5.3) (2026-05-20)


### Bug Fixes

* package.yml checks out default branch not the input tag ([166a221](https://github.com/hyperi-io/macbash/commit/166a22122d8e982eaa9564de8971669b439e4f24))

## [1.5.2](https://github.com/hyperi-io/macbash/compare/v1.5.1...v1.5.2) (2026-05-20)


### Bug Fixes

* add uninstall.sh + uninstall.ps1 to packaging framework ([c8d4065](https://github.com/hyperi-io/macbash/commit/c8d4065a5e1f798cc91ac7f93fd19f22c51324a4))
* make rendered install.sh pass macbash and shellcheck ([b7595e1](https://github.com/hyperi-io/macbash/commit/b7595e16b636f9e743c62a5e798d636989a62674))

## [1.5.1](https://github.com/hyperi-io/macbash/compare/v1.5.0...v1.5.1) (2026-05-19)


### Bug Fixes

* assert every builtin rule has both test_case sides ([a6e7136](https://github.com/hyperi-io/macbash/commit/a6e71366c7353589641e8f510f0f18e9048a53b9))
* mark fixer (-w/-o) and its runtime output as experimental ([41c957f](https://github.com/hyperi-io/macbash/commit/41c957f28f8a3f5f02d3d842bce46cfdbe6976cf))
* port cli with version metadata and examples ([2153526](https://github.com/hyperi-io/macbash/commit/2153526b0d060fba5da0616021ea9062b8ad29c9))
* port fixer with replace transform and bash -n validation ([b352c58](https://github.com/hyperi-io/macbash/commit/b352c586af4b72096f8a1a28ef5b13b83d5782e7))
* port fixer with replace transform and bash -n validation ([3f8fd84](https://github.com/hyperi-io/macbash/commit/3f8fd842a9172272ebdef5e70a5879c1deafbc07))
* port rule loader with embed parse merge validate ([af49f46](https://github.com/hyperi-io/macbash/commit/af49f462c1d30b0c9f580a01db7bf764989c3652))
* port rule type system from go macbash ([8beccd2](https://github.com/hyperi-io/macbash/commit/8beccd20e26f5b445b1f192eb21883cb9f6ec677))
* port scanner with heredoc shebang and crlf handling ([fb1a7dd](https://github.com/hyperi-io/macbash/commit/fb1a7ddcee90e9126525e81f674fb2fc5b239d7a))
* port scanner with heredoc shebang and rule-corpus tests ([fe6c16c](https://github.com/hyperi-io/macbash/commit/fe6c16cf3e7bc2d60c89f384078e7876e4c67bef))
* port text and json formatters with go-compatible json schema ([3a6e703](https://github.com/hyperi-io/macbash/commit/3a6e703f3882fa9d8920d148254336d87f19baa0))
* scaffold rust crate ([2b351f1](https://github.com/hyperi-io/macbash/commit/2b351f15ea0ee4b5150c4ddb45663a827e7af7b4))
