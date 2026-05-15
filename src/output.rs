//  Project:   macbash
//  File:      src/output.rs
//  Purpose:   Format diagnostic output as colour text or JSON
//  Language:  Rust
//
//  License:   Apache-2.0
//  Copyright: (c) 2025-2026 HYPERI PTY LIMITED

use std::io::Write;

use anstyle::{AnsiColor, Color, Style};
use serde::Serialize;
use thiserror::Error;

use crate::fixer::can_transform_grep_p;
use crate::rules::{FixType, MatchHit, RuleSet, Severity};
use crate::scanner::group_by_file;

#[derive(Debug, Error)]
pub enum OutputError {
    #[error("writing output: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialising json: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct Formatter<'a, W: Write> {
    w: W,
    use_colour: bool,
    rules: &'a RuleSet,
}

impl<'a, W: Write> Formatter<'a, W> {
    pub fn new(w: W, use_colour: bool, rules: &'a RuleSet) -> Self {
        Self {
            w,
            use_colour,
            rules,
        }
    }

    pub fn text(&mut self, matches: &[MatchHit]) -> Result<(), OutputError> {
        if matches.is_empty() {
            self.write_styled(blue(), "No issues found.")?;
            writeln!(self.w)?;
            return Ok(());
        }
        let grouped = group_by_file(matches);
        for (file, file_matches) in &grouped {
            writeln!(self.w)?;
            self.write_styled(bold(), file)?;
            writeln!(self.w)?;
            for m in file_matches {
                let rule = self.rules.rules.iter().find(|r| r.id == m.rule_id);
                let severity = rule.map(|r| r.severity).unwrap_or(Severity::Warning);
                let severity_str = match severity {
                    Severity::Error => "ERROR",
                    Severity::Warning => "WARNING",
                    Severity::Info => "INFO",
                };
                write!(self.w, "  ")?;
                self.write_styled(gray(), &format!("{}", m.line))?;
                write!(self.w, ":{}:{}: ", m.line, m.column)?;
                self.write_styled(severity_style(severity), severity_str)?;
                writeln!(self.w, " [{}]", m.rule_id)?;
                if let Some(r) = rule {
                    writeln!(self.w, "    {}", r.name)?;
                }
                write!(self.w, "    ")?;
                self.write_styled(gray(), "│ ")?;
                let highlighted = self.highlight(&m.content, &m.matched_str);
                writeln!(self.w, "{highlighted}")?;
                if let Some(r) = rule
                    && !r.fix_template.is_empty()
                {
                    let can = match r.fix_type {
                        FixType::Replace => true,
                        FixType::Transform => can_transform_match(&m.rule_id, &m.content),
                        _ => false,
                    };
                    let label = if can { "Fix" } else { "Suggest" };
                    write!(self.w, "    ")?;
                    self.write_styled(gray(), "└─ ")?;
                    writeln!(self.w, "{label}: {}", r.fix_template)?;
                    if !can && !r.why_unfixable.is_empty() {
                        write!(self.w, "    ")?;
                        self.write_styled(gray(), "   ")?;
                        write!(self.w, "Why: ")?;
                        self.write_styled(yellow(), &r.why_unfixable)?;
                        writeln!(self.w)?;
                    }
                }
            }
        }
        writeln!(self.w)?;
        let (errs, warns, infos) = count_by_severity(matches, self.rules);
        let mut parts: Vec<String> = Vec::new();
        if errs > 0 {
            parts.push(self.render(red(), &format!("{errs} error(s)")));
        }
        if warns > 0 {
            parts.push(self.render(yellow(), &format!("{warns} warning(s)")));
        }
        if infos > 0 {
            parts.push(self.render(blue(), &format!("{infos} info(s)")));
        }
        writeln!(
            self.w,
            "Found {} in {} file(s)",
            parts.join(", "),
            grouped.len()
        )?;
        Ok(())
    }

    pub fn json(&mut self, matches: &[MatchHit]) -> Result<(), OutputError> {
        #[derive(Serialize)]
        struct JsonMatch<'a> {
            file: &'a str,
            line: usize,
            column: usize,
            rule_id: &'a str,
            rule_name: &'a str,
            severity: &'a str,
            description: &'a str,
            content: &'a str,
            #[serde(rename = "matched")]
            matched_str: &'a str,
            #[serde(skip_serializing_if = "str::is_empty")]
            fix: &'a str,
            fix_type: &'a str,
            #[serde(skip_serializing_if = "str::is_empty")]
            why_unfixable: &'a str,
        }
        #[derive(Serialize)]
        struct JsonOutput<'a> {
            total_issues: usize,
            errors: usize,
            warnings: usize,
            infos: usize,
            matches: Vec<JsonMatch<'a>>,
        }
        let (errs, warns, infos) = count_by_severity(matches, self.rules);
        let items: Vec<JsonMatch<'_>> = matches
            .iter()
            .map(|m| {
                let r = self.rules.rules.iter().find(|r| r.id == m.rule_id);
                JsonMatch {
                    file: &m.file,
                    line: m.line,
                    column: m.column,
                    rule_id: &m.rule_id,
                    rule_name: r.map(|r| r.name.as_str()).unwrap_or(""),
                    severity: r.map(|r| r.severity.as_str()).unwrap_or("warning"),
                    description: r.map(|r| r.description.as_str()).unwrap_or(""),
                    content: &m.content,
                    matched_str: &m.matched_str,
                    fix: r.map(|r| r.fix_template.as_str()).unwrap_or(""),
                    fix_type: r.map(|r| r.fix_type.as_str()).unwrap_or("suggest"),
                    why_unfixable: r.map(|r| r.why_unfixable.as_str()).unwrap_or(""),
                }
            })
            .collect();
        let out = JsonOutput {
            total_issues: matches.len(),
            errors: errs,
            warnings: warns,
            infos,
            matches: items,
        };
        let pretty = serde_json::to_string_pretty(&out)?;
        self.w.write_all(pretty.as_bytes())?;
        self.w.write_all(b"\n")?;
        Ok(())
    }

    fn write_styled(&mut self, style: Style, text: &str) -> Result<(), OutputError> {
        if self.use_colour {
            write!(
                self.w,
                "{}{text}{}",
                style.render(),
                style.render_reset()
            )?;
        } else {
            self.w.write_all(text.as_bytes())?;
        }
        Ok(())
    }

    fn render(&self, style: Style, text: &str) -> String {
        if self.use_colour {
            format!("{}{text}{}", style.render(), style.render_reset())
        } else {
            text.to_string()
        }
    }

    fn highlight(&self, content: &str, matched: &str) -> String {
        if !self.use_colour || matched.is_empty() {
            return content.to_string();
        }
        content.replacen(matched, &self.render(red(), matched), 1)
    }
}

fn count_by_severity(matches: &[MatchHit], rules: &RuleSet) -> (usize, usize, usize) {
    let (mut e, mut w, mut i) = (0_usize, 0, 0);
    for m in matches {
        let sev = rules
            .rules
            .iter()
            .find(|r| r.id == m.rule_id)
            .map(|r| r.severity);
        match sev {
            Some(Severity::Error) => e += 1,
            Some(Severity::Warning) => w += 1,
            Some(Severity::Info) => i += 1,
            None => {}
        }
    }
    (e, w, i)
}

fn can_transform_match(rule_id: &str, content: &str) -> bool {
    matches!(rule_id, "grep-perl-regex" | "grep-only-matching-P")
        && can_transform_grep_p(content)
}

fn severity_style(s: Severity) -> Style {
    match s {
        Severity::Error => red(),
        Severity::Warning => yellow(),
        Severity::Info => blue(),
    }
}

const fn red() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red)))
}
const fn yellow() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)))
}
const fn blue() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::Blue)))
}
const fn gray() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlack)))
}
const fn bold() -> Style {
    Style::new().bold()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::load_builtin;
    use crate::scanner::Scanner;

    #[test]
    fn text_no_issues_says_so() {
        let rs = load_builtin().unwrap();
        let mut buf = Vec::new();
        Formatter::new(&mut buf, false, &rs).text(&[]).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("No issues"));
    }

    #[test]
    fn json_schema_field_names_match_go_oracle() {
        let rs = load_builtin().unwrap();
        let s = Scanner::new(&rs).unwrap();
        let ms = s.scan_text("sed -i 's/a/b/' f\n", "t.sh");
        let mut buf = Vec::new();
        Formatter::new(&mut buf, false, &rs).json(&ms).unwrap();
        let s = String::from_utf8(buf).unwrap();
        for key in &[
            "\"total_issues\"",
            "\"errors\"",
            "\"warnings\"",
            "\"infos\"",
            "\"matches\"",
            "\"file\"",
            "\"line\"",
            "\"column\"",
            "\"rule_id\"",
            "\"rule_name\"",
            "\"severity\"",
            "\"description\"",
            "\"content\"",
            "\"matched\"",
            "\"fix_type\"",
        ] {
            assert!(s.contains(*key), "missing key {key} in:\n{s}");
        }
    }

    #[test]
    fn json_total_issues_matches_matches_array_length() {
        let rs = load_builtin().unwrap();
        let s = Scanner::new(&rs).unwrap();
        let ms = s.scan_text(
            "#!/bin/bash\nsed -i 's/a/b/' f\ngrep -P '\\d+' f\n",
            "t.sh",
        );
        let mut buf = Vec::new();
        Formatter::new(&mut buf, false, &rs).json(&ms).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        let n = v["total_issues"].as_u64().unwrap() as usize;
        assert_eq!(n, v["matches"].as_array().unwrap().len());
        assert!(n >= 2);
    }

    #[test]
    fn text_groups_matches_under_file_headers() {
        let rs = load_builtin().unwrap();
        let s = Scanner::new(&rs).unwrap();
        let mut ms = s.scan_text("sed -i 's/a/b/' f\n", "alpha.sh");
        ms.extend(s.scan_text("grep -P '\\d+' f\n", "beta.sh"));
        let mut buf = Vec::new();
        Formatter::new(&mut buf, false, &rs).text(&ms).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("alpha.sh"));
        assert!(out.contains("beta.sh"));
        assert!(out.contains("Found"));
    }

    #[test]
    fn json_severity_field_uses_lowercase_strings() {
        let rs = load_builtin().unwrap();
        let s = Scanner::new(&rs).unwrap();
        let ms = s.scan_text("sed -i 's/a/b/' f\n", "t.sh");
        let mut buf = Vec::new();
        Formatter::new(&mut buf, false, &rs).json(&ms).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        let sev = v["matches"][0]["severity"].as_str().unwrap();
        assert!(
            sev == "error" || sev == "warning" || sev == "info",
            "unexpected severity {sev:?}"
        );
    }
}
