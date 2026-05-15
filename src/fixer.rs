//  Project:   macbash
//  File:      src/fixer.rs
//  Purpose:   Apply Replace + Transform fixes; bash -n post-validation hook
//  Language:  Rust
//
//  License:   Apache-2.0
//  Copyright: (c) 2025-2026 HYPERI PTY LIMITED

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use regex::Regex;
use thiserror::Error;

use crate::rules::{FixType, MatchHit, Rule, RuleSet};

#[derive(Debug, Error)]
pub enum FixError {
    #[error("reading {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, Default)]
pub struct FixOutcome {
    pub content: String,
    pub fixed_count: usize,
    pub unfixed_count: usize,
    pub fixes: Vec<AppliedFix>,
    /// Set by the caller after running `bash -n` validation.
    pub validation_err: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AppliedFix {
    pub line: usize,
    pub rule_id: String,
    pub original: String,
    pub fixed: String,
}

pub struct Fixer {
    rules_by_id: HashMap<String, Rule>,
    compiled: HashMap<String, Regex>,
}

impl Fixer {
    pub fn new(rs: &RuleSet) -> Self {
        Self {
            rules_by_id: rs.rules.iter().map(|r| (r.id.clone(), r.clone())).collect(),
            compiled: HashMap::new(),
        }
    }

    pub fn fix_file(
        &mut self,
        path: &Path,
        matches: &[MatchHit],
    ) -> Result<FixOutcome, FixError> {
        let text = std::fs::read_to_string(path).map_err(|e| FixError::Read {
            path: path.display().to_string(),
            source: e,
        })?;
        Ok(self.fix_text(&text, matches))
    }

    pub fn fix_text(&mut self, text: &str, matches: &[MatchHit]) -> FixOutcome {
        let mut by_line: HashMap<usize, Vec<&MatchHit>> = HashMap::new();
        for m in matches {
            by_line.entry(m.line).or_default().push(m);
        }
        // Rightmost first so column offsets remain valid as we patch.
        for v in by_line.values_mut() {
            v.sort_by(|a, b| b.column.cmp(&a.column));
        }

        let mut out_lines: Vec<String> = Vec::new();
        let mut outcome = FixOutcome::default();
        let lines: Vec<&str> = text.lines().collect();
        for (idx, line) in lines.iter().enumerate() {
            let lineno = idx + 1;
            if let Some(line_matches) = by_line.get(&lineno) {
                let (fixed, fixes, unfixed) = self.fix_line(line, line_matches);
                outcome.fixed_count += fixes.len();
                outcome.unfixed_count += unfixed;
                outcome.fixes.extend(fixes);
                out_lines.push(fixed);
            } else {
                out_lines.push((*line).to_string());
            }
        }
        outcome.content = out_lines.join("\n");
        outcome.content.push('\n');
        outcome
    }

    fn fix_line(
        &mut self,
        line: &str,
        line_matches: &[&MatchHit],
    ) -> (String, Vec<AppliedFix>, usize) {
        let mut current = line.to_string();
        let mut applied: Vec<AppliedFix> = Vec::new();
        let mut unfixed = 0_usize;
        for m in line_matches {
            let Some(rule) = self.rules_by_id.get(&m.rule_id).cloned() else {
                unfixed += 1;
                continue;
            };
            let (new_line, did_apply) = match rule.fix_type {
                FixType::Replace => {
                    let Some(re) = self.compiled_for(&rule) else {
                        unfixed += 1;
                        continue;
                    };
                    let new = re
                        .replace_all(&current, rule.fix_template.as_str())
                        .into_owned();
                    let did = new != current;
                    (new, did)
                }
                FixType::Transform => apply_transform(&current, &rule),
                _ => {
                    unfixed += 1;
                    continue;
                }
            };
            if did_apply {
                applied.push(AppliedFix {
                    line: m.line,
                    rule_id: rule.id.clone(),
                    original: current.clone(),
                    fixed: new_line.clone(),
                });
                current = new_line;
            } else {
                unfixed += 1;
            }
        }
        (current, applied, unfixed)
    }

    fn compiled_for(&mut self, rule: &Rule) -> Option<&Regex> {
        if !self.compiled.contains_key(&rule.id) {
            let re = Regex::new(&rule.pattern).ok()?;
            self.compiled.insert(rule.id.clone(), re);
        }
        self.compiled.get(&rule.id)
    }
}

/// Apply a rule-id-specific Transform.
fn apply_transform(line: &str, rule: &Rule) -> (String, bool) {
    match rule.id.as_str() {
        "grep-perl-regex" | "grep-only-matching-P" => transform_grep_p_to_e(line),
        _ => (line.to_string(), false),
    }
}

/// PCRE feature substrings with no BRE/ERE equivalent.
pub fn has_unfixable_pcre(pattern: &str) -> bool {
    const UNFIXABLE: &[&str] = &[
        r"\K", "(?=", "(?!", "(?<=", "(?<!", "(?:", "(?P<", r"\b", r"\B", "(?i)", "(?m)", "(?s)",
    ];
    UNFIXABLE.iter().any(|p| pattern.contains(p))
}

pub fn extract_grep_pattern(line: &str) -> String {
    static SINGLE: OnceLock<Regex> = OnceLock::new();
    static DOUBLE: OnceLock<Regex> = OnceLock::new();
    let single = SINGLE.get_or_init(|| {
        Regex::new(r#"grep\s+-[a-zA-Z]*P[a-zA-Z]*\s+'([^']*)'"#).unwrap()
    });
    let double = DOUBLE.get_or_init(|| {
        Regex::new(r#"grep\s+-[a-zA-Z]*P[a-zA-Z]*\s+"([^"]*)""#).unwrap()
    });
    if let Some(c) = single.captures(line) {
        return c[1].to_string();
    }
    if let Some(c) = double.captures(line) {
        return c[1].to_string();
    }
    String::new()
}

pub fn can_transform_grep_p(line: &str) -> bool {
    let p = extract_grep_pattern(line);
    !p.is_empty() && !has_unfixable_pcre(&p)
}

fn transform_grep_p_to_e(line: &str) -> (String, bool) {
    static SINGLE: OnceLock<Regex> = OnceLock::new();
    static DOUBLE: OnceLock<Regex> = OnceLock::new();
    let single = SINGLE.get_or_init(|| {
        Regex::new(r#"(grep\s+)(-[a-zA-Z]*P[a-zA-Z]*)(\s+)'([^']*)'"#).unwrap()
    });
    let double = DOUBLE.get_or_init(|| {
        Regex::new(r#"(grep\s+)(-[a-zA-Z]*P[a-zA-Z]*)(\s+)"([^"]*)""#).unwrap()
    });
    let (re, quote) = if single.is_match(line) {
        (single, '\'')
    } else if double.is_match(line) {
        (double, '"')
    } else {
        return (line.to_string(), false);
    };
    let Some(caps) = re.captures(line) else {
        return (line.to_string(), false);
    };
    let prefix = caps.get(1).unwrap().as_str();
    let flags = caps.get(2).unwrap().as_str();
    let space = caps.get(3).unwrap().as_str();
    let pattern = caps.get(4).unwrap().as_str();
    if has_unfixable_pcre(pattern) {
        return (line.to_string(), false);
    }
    let ere = pcre_to_ere(pattern);
    let new_flags = flags.replacen('P', "E", 1);
    let replacement = format!("{prefix}{new_flags}{space}{quote}{ere}{quote}");
    let new = re.replace(line, replacement.as_str()).into_owned();
    let changed = new != line;
    (new, changed)
}

fn pcre_to_ere(pattern: &str) -> String {
    const REPL: &[(&str, &str)] = &[
        (r"\d", "[0-9]"),
        (r"\D", "[^0-9]"),
        (r"\w", "[[:alnum:]_]"),
        (r"\W", "[^[:alnum:]_]"),
        (r"\s", "[[:space:]]"),
        (r"\S", "[^[:space:]]"),
    ];
    let mut s = pattern.to_string();
    for (pcre, ere) in REPL {
        s = s.replace(pcre, ere);
    }
    s
}

/// Run `bash -n` against `content` as a post-fix syntax sanity check.
/// Silently returns Ok when bash is not on PATH (typical on Windows) so
/// the fixer's output is still written; the caller decides whether the
/// missing validator is acceptable.
pub fn validate_bash_syntax(content: &str) -> Result<(), String> {
    let mut child = match Command::new("bash")
        .arg("-n")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("spawning bash: {e}")),
    };
    if let Some(stdin) = child.stdin.as_mut()
        && let Err(e) = stdin.write_all(content.as_bytes())
    {
        return Err(format!("writing to bash stdin: {e}"));
    }
    let out = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => return Err(format!("waiting on bash: {e}")),
    };
    if out.status.success() {
        return Ok(());
    }
    Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::load_builtin;
    use crate::scanner::Scanner;

    fn run_fix(input: &str) -> FixOutcome {
        let rs = load_builtin().unwrap();
        let s = Scanner::new(&rs).unwrap();
        let ms = s.scan_text(input, "t.sh");
        let mut f = Fixer::new(&rs);
        f.fix_text(input, &ms)
    }

    #[test]
    fn replaces_xargs_minus_r() {
        let out = run_fix("find . | xargs -r rm\n");
        assert!(out.content.contains("xargs rm"), "got: {:?}", out.content);
        assert!(out.fixed_count >= 1);
    }

    #[test]
    fn transforms_grep_p_with_simple_d() {
        let out = run_fix("grep -P '\\d+' f\n");
        assert!(out.content.contains("grep -E '[0-9]+'"), "got: {:?}", out.content);
        assert!(out.fixed_count >= 1);
    }

    #[test]
    fn leaves_grep_p_with_lookahead_unfixed() {
        let out = run_fix("grep -P '(?=foo)bar' f\n");
        assert!(out.content.contains("grep -P"), "got: {:?}", out.content);
        assert!(out.unfixed_count >= 1);
    }

    #[test]
    fn suggest_only_counts_as_unfixed() {
        let out = run_fix("declare -A mymap\n");
        assert_eq!(out.fixed_count, 0);
        assert!(out.unfixed_count >= 1);
    }

    #[test]
    fn pcre_to_ere_basic_substitutions() {
        assert_eq!(pcre_to_ere(r"\d+"), "[0-9]+");
        assert_eq!(pcre_to_ere(r"\w+"), "[[:alnum:]_]+");
        assert_eq!(pcre_to_ere(r"\s+"), "[[:space:]]+");
    }

    #[test]
    fn has_unfixable_pcre_detects_lookaround_and_anchors() {
        assert!(has_unfixable_pcre(r"\K"));
        assert!(has_unfixable_pcre(r"(?=foo)"));
        assert!(has_unfixable_pcre(r"\bword\b"));
        assert!(!has_unfixable_pcre(r"\d+"));
    }

    #[test]
    fn extract_grep_pattern_handles_both_quote_styles() {
        assert_eq!(extract_grep_pattern("grep -P 'foo'"), "foo");
        assert_eq!(extract_grep_pattern("grep -P \"bar\""), "bar");
        assert_eq!(extract_grep_pattern("echo nope"), "");
    }

    #[test]
    fn can_transform_grep_p_screens_out_lookahead() {
        assert!(can_transform_grep_p("grep -P '\\d+' f"));
        assert!(!can_transform_grep_p("grep -P '(?=foo)bar' f"));
        assert!(!can_transform_grep_p("echo no grep here"));
    }

    #[test]
    fn fixed_content_preserves_trailing_newline() {
        let out = run_fix("find . | xargs -r rm\n");
        assert!(out.content.ends_with('\n'));
    }

    #[test]
    fn validate_bash_syntax_accepts_valid_script() {
        // Skip if bash isn't on PATH (Windows runner without WSL).
        if Command::new("bash").arg("--version").output().is_err() {
            return;
        }
        assert!(validate_bash_syntax("echo hello\n").is_ok());
    }

    #[test]
    fn validate_bash_syntax_rejects_bad_syntax() {
        if Command::new("bash").arg("--version").output().is_err() {
            return;
        }
        let err = validate_bash_syntax("if then\n").unwrap_err();
        assert!(!err.is_empty(), "expected non-empty error from bash -n");
    }
}
