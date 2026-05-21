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
