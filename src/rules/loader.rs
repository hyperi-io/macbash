//  Project:   macbash
//  File:      src/rules/loader.rs
//  Purpose:   Load builtin and custom rule packs from YAML
//  Language:  Rust
//
//  License:   Apache-2.0
//  Copyright: (c) 2025-2026 HYPERI PTY LIMITED

use std::collections::HashMap;
use std::path::Path;

use regex::Regex;
use rust_embed::RustEmbed;
use thiserror::Error;

use super::types::{Rule, RuleSet};

#[derive(RustEmbed)]
#[folder = "src/rules/builtin/"]
struct BuiltinRules;

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("reading rules file {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("parsing YAML from {source_name}: {source}")]
    Parse {
        source_name: String,
        #[source]
        source: serde_yaml_ng::Error,
    },
    #[error("invalid rule {id} in {source_name}: {reason}")]
    Invalid {
        id: String,
        source_name: String,
        reason: String,
    },
    #[error("missing embedded rule file {0}")]
    MissingEmbedded(String),
}

/// Load all embedded builtin rule packs, concatenated into one RuleSet.
pub fn load_builtin() -> Result<RuleSet, LoadError> {
    let mut combined = RuleSet {
        version: "1.0".to_string(),
        rules: Vec::new(),
    };
    let mut filenames: Vec<String> = BuiltinRules::iter().map(|c| c.into_owned()).collect();
    filenames.sort();
    for name in filenames {
        let bytes =
            BuiltinRules::get(&name).ok_or_else(|| LoadError::MissingEmbedded(name.clone()))?;
        let mut rs = parse_ruleset(&bytes.data, &name)?;
        combined.rules.append(&mut rs.rules);
    }
    Ok(combined)
}

/// Load a custom rule set from a YAML file on disk.
pub fn load_from_file(path: &Path) -> Result<RuleSet, LoadError> {
    let data = std::fs::read(path).map_err(|e| LoadError::Read {
        path: path.display().to_string(),
        source: e,
    })?;
    parse_ruleset(&data, &path.display().to_string())
}

fn parse_ruleset(bytes: &[u8], source_name: &str) -> Result<RuleSet, LoadError> {
    let mut rs: RuleSet = serde_yaml_ng::from_slice(bytes).map_err(|e| LoadError::Parse {
        source_name: source_name.to_string(),
        source: e,
    })?;
    for r in &mut rs.rules {
        validate_rule(r, source_name)?;
    }
    Ok(rs)
}

/// Validate one rule. Mirrors the Go `validateRule` semantics:
/// - id, name, pattern required
/// - pattern (and optional negative_pattern, shebang_match) must compile as regex
/// - severity must be one of {error, warning, info} when parsed from YAML
/// - fix_type defaults to suggest via serde
///
/// Used by the loader and exposed for unit tests.
pub fn validate_rule(r: &Rule, source_name: &str) -> Result<(), LoadError> {
    if r.id.is_empty() {
        return Err(LoadError::Invalid {
            id: "<no-id>".to_string(),
            source_name: source_name.to_string(),
            reason: "rule missing id".to_string(),
        });
    }
    if r.name.is_empty() {
        return Err(LoadError::Invalid {
            id: r.id.clone(),
            source_name: source_name.to_string(),
            reason: "rule missing name".to_string(),
        });
    }
    if r.pattern.is_empty() {
        return Err(LoadError::Invalid {
            id: r.id.clone(),
            source_name: source_name.to_string(),
            reason: "rule missing pattern".to_string(),
        });
    }
    compile_pattern(&r.pattern, &r.id, source_name, "pattern")?;
    if !r.negative_pattern.is_empty() {
        compile_pattern(&r.negative_pattern, &r.id, source_name, "negative_pattern")?;
    }
    if !r.shebang_match.is_empty() {
        compile_pattern(&r.shebang_match, &r.id, source_name, "shebang_match")?;
    }
    // Severity is enforced by the serde enum; FixType too.
    let _ = (r.severity, r.fix_type);
    Ok(())
}

fn compile_pattern(
    pattern: &str,
    rule_id: &str,
    source_name: &str,
    field: &str,
) -> Result<(), LoadError> {
    Regex::new(pattern).map_err(|e| LoadError::Invalid {
        id: rule_id.to_string(),
        source_name: source_name.to_string(),
        reason: format!("invalid {field}: {e}"),
    })?;
    Ok(())
}

/// Merge rule sets in order. Later rules with the same id REPLACE earlier ones,
/// matching the Go `rules.Merge` semantics so user overrides win over builtin.
/// Nil/empty sets are skipped.
pub fn merge(sets: &[Option<&RuleSet>]) -> RuleSet {
    let mut out = RuleSet {
        version: "1.0".to_string(),
        rules: Vec::new(),
    };
    let mut idx_by_id: HashMap<String, usize> = HashMap::new();
    for set in sets.iter().flatten() {
        for r in &set.rules {
            if let Some(&i) = idx_by_id.get(&r.id) {
                out.rules[i] = r.clone();
            } else {
                idx_by_id.insert(r.id.clone(), out.rules.len());
                out.rules.push(r.clone());
            }
        }
    }
    out
}

/// Convenience wrapper for callers that only ever pass borrowed sets.
pub fn merge_some(sets: &[&RuleSet]) -> RuleSet {
    let owned: Vec<Option<&RuleSet>> = sets.iter().map(|s| Some(*s)).collect();
    merge(&owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::types::{FixType, Severity};
    use std::io::Write;

    fn write_temp(name: &str, body: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join(name);
        let mut f = std::fs::File::create(p).unwrap();
        f.write_all(body.as_bytes()).unwrap();
        dir
    }

    #[test]
    fn load_builtin_returns_seventy_three_rules() {
        let rs = load_builtin().expect("builtin rules must load");
        assert_eq!(
            rs.rules.len(),
            73,
            "rule count drifted from upstream Go binary"
        );
    }

    #[test]
    fn load_builtin_contains_expected_rule_ids() {
        let rs = load_builtin().unwrap();
        let ids: std::collections::HashSet<&str> = rs.rules.iter().map(|r| r.id.as_str()).collect();
        for expected in [
            "sed-inplace-no-backup",
            "grep-perl-regex",
            "readlink-canonicalize",
            "date-d-epoch",
            "bash4-associative-array",
        ] {
            assert!(ids.contains(expected), "missing builtin rule {expected:?}");
        }
    }

    #[test]
    fn load_builtin_every_pattern_compiles() {
        // load_builtin returns Err if any regex fails to compile; this is just
        // a louder smoke assertion.
        let _ = load_builtin().unwrap();
    }

    #[test]
    fn load_from_file_parses_minimal_yaml() {
        let body = "version: \"1.0\"\nrules:\n  - id: test-rule\n    name: Test Rule\n    description: A test rule\n    severity: warning\n    pattern: 'test\\s+pattern'\n    fix_type: suggest\n    fix_template: fixed pattern\n";
        let dir = write_temp("rules.yaml", body);
        let rs = load_from_file(&dir.path().join("rules.yaml")).unwrap();
        assert_eq!(rs.rules.len(), 1);
        assert_eq!(rs.rules[0].id, "test-rule");
        assert_eq!(rs.rules[0].severity, Severity::Warning);
    }

    #[test]
    fn load_from_file_rejects_invalid_yaml() {
        let dir = write_temp("invalid.yaml", "this is not: valid: yaml: syntax");
        let err = load_from_file(&dir.path().join("invalid.yaml")).unwrap_err();
        assert!(matches!(err, LoadError::Parse { .. }));
    }

    #[test]
    fn load_from_file_rejects_invalid_pattern() {
        let body = "version: \"1.0\"\nrules:\n  - id: bad-rule\n    name: Bad Rule\n    pattern: '[invalid'\n    severity: error\n";
        let dir = write_temp("bad.yaml", body);
        let err = load_from_file(&dir.path().join("bad.yaml")).unwrap_err();
        assert!(matches!(err, LoadError::Invalid { .. }));
    }

    #[test]
    fn load_from_file_missing_path_errors() {
        let err = load_from_file(Path::new("/nonexistent-path-macbash-test.yaml")).unwrap_err();
        assert!(matches!(err, LoadError::Read { .. }));
    }

    fn rule(id: &str, name: &str, pattern: &str, sev: Severity) -> Rule {
        Rule {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            severity: sev,
            pattern: pattern.to_string(),
            negative_pattern: String::new(),
            shebang_match: String::new(),
            fix_type: FixType::Suggest,
            fix_template: String::new(),
            why_unfixable: String::new(),
            fix_function: String::new(),
            examples: Default::default(),
            tags: vec![],
            references: vec![],
            test_cases: Default::default(),
        }
    }

    #[test]
    fn validate_rule_accepts_valid() {
        let r = rule("test", "Test", "test", Severity::Warning);
        assert!(validate_rule(&r, "t").is_ok());
    }

    #[test]
    fn validate_rule_rejects_missing_id() {
        let r = rule("", "Test", "test", Severity::Warning);
        assert!(validate_rule(&r, "t").is_err());
    }

    #[test]
    fn validate_rule_rejects_missing_name() {
        let r = rule("test", "", "test", Severity::Warning);
        assert!(validate_rule(&r, "t").is_err());
    }

    #[test]
    fn validate_rule_rejects_missing_pattern() {
        let r = rule("test", "Test", "", Severity::Warning);
        assert!(validate_rule(&r, "t").is_err());
    }

    #[test]
    fn validate_rule_rejects_invalid_regex() {
        let r = rule("test", "Test", "[invalid", Severity::Warning);
        assert!(validate_rule(&r, "t").is_err());
    }

    #[test]
    fn validate_rule_rejects_invalid_negative_pattern() {
        let mut r = rule("test", "Test", "test", Severity::Warning);
        r.negative_pattern = "[bad".to_string();
        assert!(validate_rule(&r, "t").is_err());
    }

    #[test]
    fn validate_rule_rejects_invalid_shebang_pattern() {
        let mut r = rule("test", "Test", "test", Severity::Warning);
        r.shebang_match = "[bad".to_string();
        assert!(validate_rule(&r, "t").is_err());
    }

    #[test]
    fn merge_later_overrides_earlier() {
        let a = RuleSet {
            version: "1.0".to_string(),
            rules: vec![
                rule("rule1", "Rule 1", "p1", Severity::Warning),
                rule("rule2", "Rule 2", "p2", Severity::Warning),
            ],
        };
        let b = RuleSet {
            version: "1.0".to_string(),
            rules: vec![
                rule("rule2", "Rule 2 Override", "p2-new", Severity::Warning),
                rule("rule3", "Rule 3", "p3", Severity::Warning),
            ],
        };
        let m = merge_some(&[&a, &b]);
        assert_eq!(m.rules.len(), 3);
        let r2 = m.rules.iter().find(|r| r.id == "rule2").unwrap();
        assert_eq!(r2.name, "Rule 2 Override");
        assert_eq!(r2.pattern, "p2-new");
    }

    #[test]
    fn merge_skips_nil_sets() {
        let a = RuleSet {
            version: "1.0".to_string(),
            rules: vec![rule("rule1", "Rule 1", "p1", Severity::Warning)],
        };
        let m = merge(&[None, Some(&a), None]);
        assert_eq!(m.rules.len(), 1);
        assert_eq!(m.rules[0].id, "rule1");
    }

    #[test]
    fn merge_empty_input_yields_empty_set() {
        let m: RuleSet = merge(&[]);
        assert!(m.rules.is_empty());
        assert_eq!(m.version, "1.0");
    }
}
