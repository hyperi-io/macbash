//  Project:   macbash
//  File:      src/rules/types.rs
//  Purpose:   Rule type definitions
//  Language:  Rust
//
//  License:   Apache-2.0
//  Copyright: (c) 2025-2026 HYPERI PTY LIMITED

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    pub fn order(self) -> u8 {
        match self {
            Severity::Info => 0,
            Severity::Warning => 1,
            Severity::Error => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FixType {
    Replace,
    Suggest,
    Function,
    Transform,
}

impl FixType {
    pub fn as_str(self) -> &'static str {
        match self {
            FixType::Replace => "replace",
            FixType::Suggest => "suggest",
            FixType::Function => "function",
            FixType::Transform => "transform",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Examples {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bad: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub good: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestCases {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub should_match: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub should_not_match: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_severity")]
    pub severity: Severity,
    pub pattern: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub negative_pattern: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub shebang_match: String,
    #[serde(default = "default_fix_type")]
    pub fix_type: FixType,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub fix_template: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub why_unfixable: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub fix_function: String,
    #[serde(default)]
    pub examples: Examples,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,
    #[serde(default)]
    pub test_cases: TestCases,
}

fn default_severity() -> Severity {
    Severity::Warning
}

fn default_fix_type() -> FixType {
    FixType::Suggest
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSet {
    pub version: String,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone)]
pub struct MatchHit {
    pub rule_id: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub content: String,
    pub matched_str: String,
    pub fixed_str: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_deserialises_lowercase() {
        let s: Severity = serde_yaml_ng::from_str("error").unwrap();
        assert_eq!(s, Severity::Error);
        let s: Severity = serde_yaml_ng::from_str("warning").unwrap();
        assert_eq!(s, Severity::Warning);
        let s: Severity = serde_yaml_ng::from_str("info").unwrap();
        assert_eq!(s, Severity::Info);
    }

    #[test]
    fn fix_type_deserialises_lowercase() {
        let f: FixType = serde_yaml_ng::from_str("replace").unwrap();
        assert_eq!(f, FixType::Replace);
        let f: FixType = serde_yaml_ng::from_str("suggest").unwrap();
        assert_eq!(f, FixType::Suggest);
        let f: FixType = serde_yaml_ng::from_str("transform").unwrap();
        assert_eq!(f, FixType::Transform);
        let f: FixType = serde_yaml_ng::from_str("function").unwrap();
        assert_eq!(f, FixType::Function);
    }

    #[test]
    fn rule_deserialises_minimal_yaml() {
        let yaml = "id: test-rule\nname: Test\ndescription: A test\nseverity: error\npattern: 'foo'\nfix_type: suggest\n";
        let r: Rule = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(r.id, "test-rule");
        assert_eq!(r.severity, Severity::Error);
        assert_eq!(r.fix_type, FixType::Suggest);
    }

    #[test]
    fn rule_defaults_severity_warning_when_missing() {
        let yaml = "id: r\nname: R\npattern: 'x'\n";
        let r: Rule = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(r.severity, Severity::Warning);
        assert_eq!(r.fix_type, FixType::Suggest);
    }

    #[test]
    fn ruleset_deserialises_with_version_and_rules() {
        let yaml = "version: \"1.0\"\nrules:\n  - id: r1\n    name: One\n    description: d\n    severity: warning\n    pattern: 'x'\n    fix_type: suggest\n";
        let rs: RuleSet = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(rs.version, "1.0");
        assert_eq!(rs.rules.len(), 1);
    }

    #[test]
    fn severity_order_orders_info_warning_error() {
        assert!(Severity::Info.order() < Severity::Warning.order());
        assert!(Severity::Warning.order() < Severity::Error.order());
    }

    #[test]
    fn severity_as_str_round_trip() {
        for s in [Severity::Error, Severity::Warning, Severity::Info] {
            assert_eq!(serde_yaml_ng::from_str::<Severity>(s.as_str()).unwrap(), s);
        }
    }

    #[test]
    fn fix_type_as_str_round_trip() {
        for f in [FixType::Replace, FixType::Suggest, FixType::Function, FixType::Transform] {
            assert_eq!(serde_yaml_ng::from_str::<FixType>(f.as_str()).unwrap(), f);
        }
    }

    #[test]
    fn rule_examples_optional() {
        let yaml = "id: r\nname: R\npattern: 'x'\nexamples:\n  bad: 'no'\n  good: 'yes'\n";
        let r: Rule = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(r.examples.bad, "no");
        assert_eq!(r.examples.good, "yes");
    }

    #[test]
    fn rule_test_cases_optional() {
        let yaml = "id: r\nname: R\npattern: 'x'\ntest_cases:\n  should_match:\n    - 'a'\n  should_not_match:\n    - 'b'\n";
        let r: Rule = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(r.test_cases.should_match, vec!["a"]);
        assert_eq!(r.test_cases.should_not_match, vec!["b"]);
    }
}
