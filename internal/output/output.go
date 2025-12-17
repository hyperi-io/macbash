// Project:   macbash
// File:      internal/output/output.go
// Purpose:   Format and display scan results
// Language:  Go
//
// License:   Apache-2.0
// Copyright: (c) 2025 HyperSec Pty Ltd

package output

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"strings"

	"github.com/hypersec-io/macbash/internal/fixer"
	"github.com/hypersec-io/macbash/internal/rules"
)

type Formatter struct {
	writer    io.Writer
	useColors bool
}

const (
	colorReset  = "\033[0m"
	colorRed    = "\033[31m"
	colorYellow = "\033[33m"
	colorBlue   = "\033[34m"
	colorGray   = "\033[90m"
	colorBold   = "\033[1m"
)

func New(w io.Writer) *Formatter {
	useColors := false
	if f, ok := w.(*os.File); ok {
		stat, _ := f.Stat()
		useColors = (stat.Mode() & os.ModeCharDevice) != 0
	}

	return &Formatter{
		writer:    w,
		useColors: useColors,
	}
}

func (f *Formatter) Text(matches []rules.Match) {
	if len(matches) == 0 {
		fmt.Fprintln(f.writer, f.color(colorBlue, "No issues found."))
		return
	}

	grouped := groupByFile(matches)

	for file, fileMatches := range grouped {
		fmt.Fprintf(f.writer, "\n%s\n", f.color(colorBold, file))

		for _, m := range fileMatches {
			severityColor := f.severityColor(m.Rule.Severity)
			severityStr := strings.ToUpper(string(m.Rule.Severity))

			fmt.Fprintf(f.writer, "  %s:%d:%d: %s [%s]\n",
				f.color(colorGray, fmt.Sprintf("%d", m.Line)),
				m.Line,
				m.Column,
				f.color(severityColor, severityStr),
				m.Rule.ID,
			)

			fmt.Fprintf(f.writer, "    %s\n", m.Rule.Name)
			fmt.Fprintf(f.writer, "    %s%s\n",
				f.color(colorGray, "│ "),
				highlightMatch(m.Content, m.MatchedStr, f.useColors),
			)

			if m.Rule.FixTemplate != "" {
				canAutoFix := m.Rule.FixType == rules.FixReplace
				if m.Rule.FixType == rules.FixTransform {
					canAutoFix = canTransformMatch(&m)
				}

				fixType := "Fix"
				if !canAutoFix {
					fixType = "Suggest"
				}
				fmt.Fprintf(f.writer, "    %s%s: %s\n",
					f.color(colorGray, "└─ "),
					fixType,
					m.Rule.FixTemplate,
				)

				if !canAutoFix && m.Rule.WhyUnfixable != "" {
					fmt.Fprintf(f.writer, "    %sWhy: %s\n",
						f.color(colorGray, "   "),
						f.color(colorYellow, m.Rule.WhyUnfixable),
					)
				}
			}
		}
	}

	fmt.Fprintln(f.writer)
	errors, warnings, infos := countBySeverity(matches)

	parts := []string{}
	if errors > 0 {
		parts = append(parts, f.color(colorRed, fmt.Sprintf("%d error(s)", errors)))
	}
	if warnings > 0 {
		parts = append(parts, f.color(colorYellow, fmt.Sprintf("%d warning(s)", warnings)))
	}
	if infos > 0 {
		parts = append(parts, f.color(colorBlue, fmt.Sprintf("%d info(s)", infos)))
	}

	fmt.Fprintf(f.writer, "Found %s in %d file(s)\n",
		strings.Join(parts, ", "),
		len(grouped),
	)
}

func (f *Formatter) JSON(matches []rules.Match) error {
	type jsonMatch struct {
		File         string `json:"file"`
		Line         int    `json:"line"`
		Column       int    `json:"column"`
		RuleID       string `json:"rule_id"`
		RuleName     string `json:"rule_name"`
		Severity     string `json:"severity"`
		Description  string `json:"description"`
		Content      string `json:"content"`
		MatchedStr   string `json:"matched"`
		Fix          string `json:"fix,omitempty"`
		FixType      string `json:"fix_type"`
		WhyUnfixable string `json:"why_unfixable,omitempty"`
	}

	type jsonOutput struct {
		TotalIssues int         `json:"total_issues"`
		Errors      int         `json:"errors"`
		Warnings    int         `json:"warnings"`
		Infos       int         `json:"infos"`
		Matches     []jsonMatch `json:"matches"`
	}

	errors, warnings, infos := countBySeverity(matches)

	output := jsonOutput{
		TotalIssues: len(matches),
		Errors:      errors,
		Warnings:    warnings,
		Infos:       infos,
		Matches:     make([]jsonMatch, len(matches)),
	}

	for i, m := range matches {
		output.Matches[i] = jsonMatch{
			File:         m.File,
			Line:         m.Line,
			Column:       m.Column,
			RuleID:       m.Rule.ID,
			RuleName:     m.Rule.Name,
			Severity:     string(m.Rule.Severity),
			Description:  m.Rule.Description,
			Content:      m.Content,
			MatchedStr:   m.MatchedStr,
			Fix:          m.Rule.FixTemplate,
			FixType:      string(m.Rule.FixType),
			WhyUnfixable: m.Rule.WhyUnfixable,
		}
	}

	encoder := json.NewEncoder(f.writer)
	encoder.SetIndent("", "  ")
	return encoder.Encode(output)
}

func (f *Formatter) color(code, text string) string {
	if !f.useColors {
		return text
	}
	return code + text + colorReset
}

func (f *Formatter) severityColor(s rules.Severity) string {
	switch s {
	case rules.SeverityError:
		return colorRed
	case rules.SeverityWarning:
		return colorYellow
	case rules.SeverityInfo:
		return colorBlue
	default:
		return colorReset
	}
}

func groupByFile(matches []rules.Match) map[string][]rules.Match {
	grouped := make(map[string][]rules.Match)
	for _, m := range matches {
		grouped[m.File] = append(grouped[m.File], m)
	}
	return grouped
}

func countBySeverity(matches []rules.Match) (errors, warnings, infos int) {
	for _, m := range matches {
		switch m.Rule.Severity {
		case rules.SeverityError:
			errors++
		case rules.SeverityWarning:
			warnings++
		case rules.SeverityInfo:
			infos++
		}
	}
	return
}

func highlightMatch(content, matched string, useColors bool) string {
	if !useColors || matched == "" {
		return content
	}
	return strings.Replace(content, matched, colorRed+matched+colorReset, 1)
}

func canTransformMatch(m *rules.Match) bool {
	switch m.Rule.ID {
	case "grep-perl-regex", "grep-only-matching-P":
		return fixer.CanTransformGrepP(m.Content)
	default:
		return false
	}
}
