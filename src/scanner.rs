//  Project:   macbash
//  File:      src/scanner.rs
//  Purpose:   Line-by-line rule matcher with shebang gating, here-doc skipping
//  Language:  Rust
//
//  License:   Apache-2.0
//  Copyright: (c) 2025-2026 HYPERI PTY LIMITED

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;
use thiserror::Error;

use crate::rules::{FixType, MatchHit, RuleSet, Severity};

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("compiling pattern for rule {rule_id}: {source}")]
    BadPattern {
        rule_id: String,
        #[source]
        source: regex::Error,
    },
    #[error("reading {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

struct CompiledRule {
    rule_id: String,
    rule_pattern_starts_caret_hash: bool,
    pattern: Regex,
    negative: Option<Regex>,
    shebang: Option<Regex>,
    fix_type: FixType,
    fix_template: String,
}

pub struct Scanner {
    compiled: Vec<CompiledRule>,
}

impl Scanner {
    /// Compile every rule in the set. Errors out on the first bad regex.
    pub fn new(rs: &RuleSet) -> Result<Self, ScanError> {
        let mut compiled = Vec::with_capacity(rs.rules.len());
        for r in &rs.rules {
            let pattern = Regex::new(&r.pattern).map_err(|e| ScanError::BadPattern {
                rule_id: r.id.clone(),
                source: e,
            })?;
            let negative = if r.negative_pattern.is_empty() {
                None
            } else {
                Some(
                    Regex::new(&r.negative_pattern).map_err(|e| ScanError::BadPattern {
                        rule_id: r.id.clone(),
                        source: e,
                    })?,
                )
            };
            let shebang = if r.shebang_match.is_empty() {
                None
            } else {
                Some(
                    Regex::new(&r.shebang_match).map_err(|e| ScanError::BadPattern {
                        rule_id: r.id.clone(),
                        source: e,
                    })?,
                )
            };
            compiled.push(CompiledRule {
                rule_id: r.id.clone(),
                rule_pattern_starts_caret_hash: r.pattern.starts_with("^#"),
                pattern,
                negative,
                shebang,
                fix_type: r.fix_type,
                fix_template: r.fix_template.clone(),
            });
        }
        Ok(Self { compiled })
    }

    pub fn rule_count(&self) -> usize {
        self.compiled.len()
    }

    pub fn scan_file(&self, path: &Path) -> Result<Vec<MatchHit>, ScanError> {
        let text = std::fs::read_to_string(path).map_err(|e| ScanError::Read {
            path: path.display().to_string(),
            source: e,
        })?;
        Ok(self.scan_text(&text, &path.display().to_string()))
    }

    pub fn scan_text(&self, text: &str, source: &str) -> Vec<MatchHit> {
        let mut out: Vec<MatchHit> = Vec::new();
        let mut shebang_owned: String = String::new();
        let mut in_heredoc = false;
        let mut heredoc_delim = String::new();

        for (idx, raw) in text.lines().enumerate() {
            // .lines() already handles \r\n by stripping the \r; line numbers
            // are 1-based for users.
            let line = raw;
            let line_num = idx + 1;

            if line_num == 1 && line.starts_with("#!") {
                shebang_owned = line.to_string();
            }

            if in_heredoc {
                if line.trim() == heredoc_delim {
                    in_heredoc = false;
                    heredoc_delim.clear();
                }
                continue;
            }
            if let Some(d) = detect_heredoc(line) {
                in_heredoc = true;
                heredoc_delim = d;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            for cr in &self.compiled {
                if let Some(sh) = &cr.shebang
                    && !sh.is_match(&shebang_owned)
                {
                    continue;
                }
                if trimmed.starts_with('#') && !cr.rule_pattern_starts_caret_hash {
                    continue;
                }
                let Some(m) = cr.pattern.find(line) else {
                    continue;
                };
                if let Some(neg) = &cr.negative
                    && neg.is_match(line)
                {
                    continue;
                }
                let mut hit = MatchHit {
                    rule_id: cr.rule_id.clone(),
                    file: source.to_string(),
                    line: line_num,
                    column: m.start() + 1,
                    content: line.to_string(),
                    matched_str: line[m.start()..m.end()].to_string(),
                    fixed_str: None,
                };
                if cr.fix_type == FixType::Replace && !cr.fix_template.is_empty() {
                    hit.fixed_str = Some(
                        cr.pattern
                            .replace_all(line, cr.fix_template.as_str())
                            .into_owned(),
                    );
                }
                out.push(hit);
            }
        }
        out
    }

    pub fn scan_files<'a>(
        &self,
        paths: impl IntoIterator<Item = &'a Path>,
    ) -> Result<Vec<MatchHit>, ScanError> {
        let mut all = Vec::new();
        for p in paths {
            all.extend(self.scan_file(p)?);
        }
        Ok(all)
    }
}

/// Filter matches to those at or above `min`. The Go implementation uses
/// match.Rule.Severity directly; here we look the rule up by id from the
/// RuleSet to avoid stuffing a borrowed reference into MatchHit.
pub fn filter_by_severity(
    matches: Vec<MatchHit>,
    min: Severity,
    rs: &RuleSet,
) -> Vec<MatchHit> {
    let min_rank = min.order();
    let sev: BTreeMap<&str, Severity> = rs
        .rules
        .iter()
        .map(|r| (r.id.as_str(), r.severity))
        .collect();
    matches
        .into_iter()
        .filter(|m| {
            sev.get(m.rule_id.as_str())
                .copied()
                .map(|s| s.order())
                .unwrap_or(0)
                >= min_rank
        })
        .collect()
}

/// Return true if any match references a rule with severity = Error.
pub fn has_errors(matches: &[MatchHit], rs: &RuleSet) -> bool {
    let by_id: BTreeMap<&str, Severity> = rs
        .rules
        .iter()
        .map(|r| (r.id.as_str(), r.severity))
        .collect();
    matches
        .iter()
        .any(|m| by_id.get(m.rule_id.as_str()).copied() == Some(Severity::Error))
}

/// Group matches by file name (BTreeMap for stable iteration order).
pub fn group_by_file(matches: &[MatchHit]) -> BTreeMap<String, Vec<&MatchHit>> {
    let mut g: BTreeMap<String, Vec<&MatchHit>> = BTreeMap::new();
    for m in matches {
        g.entry(m.file.clone()).or_default().push(m);
    }
    g
}

/// Detect here-doc start; returns the delimiter or None.
/// Matches the Go regex `<<-?\s*['"]?(\w+)['"]?`.
pub fn detect_heredoc(line: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r#"<<-?\s*['"]?(\w+)['"]?"#).unwrap());
    re.captures(line).map(|c| c[1].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{load_builtin, FixType, Rule, RuleSet, Severity};

    fn rule(id: &str, pattern: &str, sev: Severity) -> Rule {
        Rule {
            id: id.to_string(),
            name: id.to_string(),
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
    fn new_compiles_valid_pattern() {
        let rs = RuleSet {
            version: "1.0".to_string(),
            rules: vec![rule("test-rule", r"echo\s+-e", Severity::Warning)],
        };
        let s = Scanner::new(&rs).unwrap();
        assert_eq!(s.rule_count(), 1);
    }

    #[test]
    fn new_rejects_invalid_pattern() {
        let rs = RuleSet {
            version: "1.0".to_string(),
            rules: vec![rule("bad-rule", "[invalid", Severity::Warning)],
        };
        assert!(Scanner::new(&rs).is_err());
    }

    #[test]
    fn scan_text_finds_echo_e_and_grep_p() {
        let rs = RuleSet {
            version: "1.0".to_string(),
            rules: vec![
                rule("echo-escape", r"echo\s+-e\s", Severity::Warning),
                rule("grep-perl", r"grep\s+-P\s", Severity::Error),
            ],
        };
        let s = Scanner::new(&rs).unwrap();
        let text = "#!/bin/bash\necho -e \"hello\\nworld\"\ngrep -P '\\d+' file.txt\n";
        let matches = s.scan_text(text, "test.sh");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].rule_id, "echo-escape");
        assert_eq!(matches[0].line, 2);
        assert_eq!(matches[1].rule_id, "grep-perl");
        assert_eq!(matches[1].line, 3);
    }

    #[test]
    fn negative_pattern_suppresses_match() {
        let mut r = rule("realpath-unchecked", r"\brealpath\s+", Severity::Warning);
        r.negative_pattern = r"command\s+-v\s+realpath".to_string();
        let rs = RuleSet {
            version: "1.0".to_string(),
            rules: vec![r],
        };
        let s = Scanner::new(&rs).unwrap();
        let text = "#!/bin/bash\n# Check if realpath exists before using\ncommand -v realpath && realpath /some/path\nrealpath /another/path\n";
        let matches = s.scan_text(text, "test.sh");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line, 4);
    }

    #[test]
    fn shebang_match_gates_rule_to_bin_sh() {
        let mut r = rule("posix-double-bracket", r"\[\[", Severity::Error);
        r.shebang_match = r"^#!/bin/sh".to_string();
        let rs = RuleSet {
            version: "1.0".to_string(),
            rules: vec![r],
        };
        let s = Scanner::new(&rs).unwrap();

        let sh_hits = s.scan_text(
            "#!/bin/sh\nif [[ -f foo ]]; then echo yes; fi\n",
            "sh.sh",
        );
        assert_eq!(sh_hits.len(), 1, "[[ in /bin/sh must match");

        let bash_hits = s.scan_text(
            "#!/bin/bash\nif [[ -f foo ]]; then echo yes; fi\n",
            "bash.sh",
        );
        assert_eq!(bash_hits.len(), 0, "[[ in /bin/bash must be skipped");

        let env_hits = s.scan_text(
            "#!/usr/bin/env bash\nif [[ -f foo ]]; then echo yes; fi\n",
            "env.sh",
        );
        assert_eq!(env_hits.len(), 0, "[[ in env bash must be skipped");
    }

    #[test]
    fn heredoc_lines_are_skipped() {
        let rs = RuleSet {
            version: "1.0".to_string(),
            rules: vec![
                rule("echo-escape", r"echo\s+-e\s", Severity::Warning),
                rule("grep-perl", r"grep\s+-P\s", Severity::Error),
            ],
        };
        let s = Scanner::new(&rs).unwrap();
        let text = concat!(
            "#!/bin/bash\n",
            "cat <<EOF\n",
            "echo -e \"this is inside a heredoc and should be skipped\"\n",
            "grep -P '\\d+' not-a-real-command\n",
            "EOF\n",
            "echo -e \"this is real code and should be caught\"\n",
        );
        let matches = s.scan_text(text, "hd.sh");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line, 6);
    }

    #[test]
    fn timeout_false_positive_suppressed_by_negative_pattern() {
        let mut r = rule("timeout-command", r"\btimeout\s+(\d)", Severity::Error);
        r.negative_pattern = "-timeout".to_string();
        let rs = RuleSet {
            version: "1.0".to_string(),
            rules: vec![r],
        };
        let s = Scanner::new(&rs).unwrap();
        let text = concat!(
            "#!/bin/bash\n",
            "timeout 5 some-command\n",
            "golangci-lint run --timeout 5m\n",
            "curl --connect-timeout 30 https://example.com\n",
        );
        let matches = s.scan_text(text, "t.sh");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line, 2);
    }

    #[test]
    fn detect_heredoc_table() {
        let cases = [
            ("cat <<EOF", Some("EOF")),
            ("cat <<'EOF'", Some("EOF")),
            ("cat <<\"EOF\"", Some("EOF")),
            ("cat <<-EOF", Some("EOF")),
            ("cat <<-'MARKER'", Some("MARKER")),
            ("echo hello", None),
        ];
        for (line, want) in cases {
            let got = detect_heredoc(line);
            assert_eq!(
                got.as_deref(),
                want,
                "detect_heredoc({line:?}) returned {got:?}"
            );
        }
    }

    #[test]
    fn builtin_rules_all_compile() {
        let rs = load_builtin().unwrap();
        let s = Scanner::new(&rs).unwrap();
        assert!(s.rule_count() > 0);
        assert_eq!(s.rule_count(), rs.rules.len());
    }

    #[test]
    fn filter_by_severity_counts() {
        let rs = RuleSet {
            version: "1.0".to_string(),
            rules: vec![
                rule("i1", "x", Severity::Info),
                rule("w1", "x", Severity::Warning),
                rule("e1", "x", Severity::Error),
                rule("i2", "x", Severity::Info),
            ],
        };
        let matches: Vec<MatchHit> = rs
            .rules
            .iter()
            .map(|r| MatchHit {
                rule_id: r.id.clone(),
                file: "t.sh".to_string(),
                line: 1,
                column: 1,
                content: String::new(),
                matched_str: String::new(),
                fixed_str: None,
            })
            .collect();
        assert_eq!(filter_by_severity(matches.clone(), Severity::Info, &rs).len(), 4);
        assert_eq!(filter_by_severity(matches.clone(), Severity::Warning, &rs).len(), 2);
        assert_eq!(filter_by_severity(matches, Severity::Error, &rs).len(), 1);
    }

    #[test]
    fn has_errors_table() {
        let rs = RuleSet {
            version: "1.0".to_string(),
            rules: vec![
                rule("w", "x", Severity::Warning),
                rule("e", "x", Severity::Error),
            ],
        };
        let warn_only = vec![MatchHit {
            rule_id: "w".to_string(),
            file: "t.sh".to_string(),
            line: 1,
            column: 1,
            content: String::new(),
            matched_str: String::new(),
            fixed_str: None,
        }];
        let with_error = vec![
            warn_only[0].clone(),
            MatchHit {
                rule_id: "e".to_string(),
                file: "t.sh".to_string(),
                line: 2,
                column: 1,
                content: String::new(),
                matched_str: String::new(),
                fixed_str: None,
            },
        ];
        assert!(!has_errors(&[], &rs));
        assert!(!has_errors(&warn_only, &rs));
        assert!(has_errors(&with_error, &rs));
    }

    #[test]
    fn columns_are_one_based() {
        let rs = RuleSet {
            version: "1.0".to_string(),
            rules: vec![rule("sed-inplace", r"sed\s+-i\s+[^.]", Severity::Error)],
        };
        let s = Scanner::new(&rs).unwrap();
        let matches = s.scan_text("   sed -i 's/a/b/' f\n", "t.sh");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].column, 4);
    }

    #[test]
    fn comment_lines_skipped_unless_pattern_starts_caret_hash() {
        let rs = RuleSet {
            version: "1.0".to_string(),
            rules: vec![
                rule("sed-inplace", r"sed\s+-i\s+[^.]", Severity::Error),
                rule("shebang-bin-bash", r"^#!/bin/bash\s*$", Severity::Info),
            ],
        };
        let s = Scanner::new(&rs).unwrap();
        let m1 = s.scan_text("# sed -i 's/a/b/' file\n", "t.sh");
        assert!(!m1.iter().any(|h| h.rule_id == "sed-inplace"));
        let m2 = s.scan_text("#!/bin/bash\n", "t.sh");
        assert!(m2.iter().any(|h| h.rule_id == "shebang-bin-bash"));
    }

    #[test]
    fn crlf_inputs_yield_correct_line_numbers() {
        let rs = RuleSet {
            version: "1.0".to_string(),
            rules: vec![rule("echo-escape", r"echo\s+-e\s", Severity::Warning)],
        };
        let s = Scanner::new(&rs).unwrap();
        let matches = s.scan_text("#!/bin/bash\r\necho -e foo\r\n", "win.sh");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line, 2);
    }

    #[test]
    fn group_by_file_groups_correctly() {
        let m: Vec<MatchHit> = vec![
            MatchHit {
                rule_id: "r".into(),
                file: "a.sh".into(),
                line: 1,
                column: 1,
                content: String::new(),
                matched_str: String::new(),
                fixed_str: None,
            },
            MatchHit {
                rule_id: "r".into(),
                file: "b.sh".into(),
                line: 1,
                column: 1,
                content: String::new(),
                matched_str: String::new(),
                fixed_str: None,
            },
            MatchHit {
                rule_id: "r".into(),
                file: "a.sh".into(),
                line: 2,
                column: 1,
                content: String::new(),
                matched_str: String::new(),
                fixed_str: None,
            },
        ];
        let g = group_by_file(&m);
        assert_eq!(g.len(), 2);
        assert_eq!(g["a.sh"].len(), 2);
        assert_eq!(g["b.sh"].len(), 1);
    }

    /// Wrap a single-line example into a full script body matching the rule's
    /// shebang gate. Rules whose pattern itself targets the shebang line are
    /// fed verbatim (a synthetic shebang would self-match those rules).
    fn synthesise_script(rule: &Rule, example: &str) -> String {
        if rule.pattern.starts_with("^#") {
            // Shebang-pattern rule — feed example directly as the shebang line.
            return format!("{example}\n");
        }
        let header = if rule.shebang_match.is_empty() {
            "#!/bin/bash"
        } else if rule.shebang_match.contains("sh") && !rule.shebang_match.contains("bash") {
            "#!/bin/sh"
        } else {
            "#!/bin/bash"
        };
        format!("{header}\n{example}\n")
    }

    #[test]
    fn every_should_match_example_matches_its_own_rule() {
        let rs = load_builtin().unwrap();
        let s = Scanner::new(&rs).unwrap();
        let mut failures: Vec<String> = Vec::new();
        for rule in &rs.rules {
            for ex in &rule.test_cases.should_match {
                let text = synthesise_script(rule, ex);
                let hits = s.scan_text(&text, "ex.sh");
                if !hits.iter().any(|h| h.rule_id == rule.id) {
                    failures.push(format!("rule {} did not match: {ex:?}", rule.id));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "should_match failures ({}):\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    #[test]
    fn no_should_not_match_example_matches_its_own_rule() {
        let rs = load_builtin().unwrap();
        let s = Scanner::new(&rs).unwrap();
        let mut failures: Vec<String> = Vec::new();
        for rule in &rs.rules {
            for ex in &rule.test_cases.should_not_match {
                let text = synthesise_script(rule, ex);
                let hits = s.scan_text(&text, "ex.sh");
                if hits.iter().any(|h| h.rule_id == rule.id) {
                    failures.push(format!("rule {} unexpectedly matched: {ex:?}", rule.id));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "should_not_match failures ({}):\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}
