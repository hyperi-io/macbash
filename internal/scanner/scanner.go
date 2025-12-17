// Project:   macbash
// File:      internal/scanner/scanner.go
// Purpose:   Scan files for rule violations
// Language:  Go
//
// License:   Apache-2.0
// Copyright: (c) 2025 HyperSec Pty Ltd

package scanner

import (
	"bufio"
	"os"
	"regexp"
	"strings"

	"github.com/hypersec-io/macbash/internal/rules"
)

type Scanner struct {
	rules    []rules.Rule
	compiled map[string]*compiledRule
}

type compiledRule struct {
	rule     *rules.Rule
	pattern  *regexp.Regexp
	negative *regexp.Regexp
}

func New(ruleSet *rules.RuleSet) (*Scanner, error) {
	s := &Scanner{
		rules:    ruleSet.Rules,
		compiled: make(map[string]*compiledRule),
	}

	// Pre-compile all patterns
	for i := range s.rules {
		r := &s.rules[i]
		cr := &compiledRule{rule: r}

		var err error
		cr.pattern, err = regexp.Compile(r.Pattern)
		if err != nil {
			return nil, err
		}

		if r.NegativePattern != "" {
			cr.negative, err = regexp.Compile(r.NegativePattern)
			if err != nil {
				return nil, err
			}
		}

		s.compiled[r.ID] = cr
	}

	return s, nil
}

func (s *Scanner) ScanFile(path string) ([]rules.Match, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	var matches []rules.Match
	scanner := bufio.NewScanner(file)
	lineNum := 0

	for scanner.Scan() {
		lineNum++
		line := scanner.Text()

		// Skip empty lines and pure comment lines (for efficiency)
		trimmed := strings.TrimSpace(line)
		if trimmed == "" {
			continue
		}

		// Check each rule
		for _, cr := range s.compiled {
			// Skip if line is a comment and pattern doesn't explicitly target comments
			if strings.HasPrefix(trimmed, "#") && !strings.HasPrefix(cr.rule.Pattern, "^#") {
				continue
			}

			// Check if pattern matches
			loc := cr.pattern.FindStringIndex(line)
			if loc == nil {
				continue
			}

			// Check negative pattern (exclusion)
			if cr.negative != nil && cr.negative.MatchString(line) {
				continue
			}

			matchedStr := line[loc[0]:loc[1]]

			match := rules.Match{
				Rule:       cr.rule,
				File:       path,
				Line:       lineNum,
				Column:     loc[0] + 1,
				Content:    line,
				MatchedStr: matchedStr,
			}

			// Generate fix if applicable
			if cr.rule.FixType == rules.FixReplace && cr.rule.FixTemplate != "" {
				match.FixedStr = cr.pattern.ReplaceAllString(line, cr.rule.FixTemplate)
			}

			matches = append(matches, match)
		}
	}

	if err := scanner.Err(); err != nil {
		return nil, err
	}

	return matches, nil
}

func (s *Scanner) ScanFiles(paths []string) ([]rules.Match, error) {
	var allMatches []rules.Match

	for _, path := range paths {
		matches, err := s.ScanFile(path)
		if err != nil {
			return nil, err
		}
		allMatches = append(allMatches, matches...)
	}

	return allMatches, nil
}

func FilterBySeverity(matches []rules.Match, minSeverity rules.Severity) []rules.Match {
	severityOrder := map[rules.Severity]int{
		rules.SeverityInfo:    0,
		rules.SeverityWarning: 1,
		rules.SeverityError:   2,
	}

	minLevel := severityOrder[minSeverity]
	var filtered []rules.Match

	for _, m := range matches {
		if severityOrder[m.Rule.Severity] >= minLevel {
			filtered = append(filtered, m)
		}
	}

	return filtered
}

func HasErrors(matches []rules.Match) bool {
	for _, m := range matches {
		if m.Rule.Severity == rules.SeverityError {
			return true
		}
	}
	return false
}

func GroupByFile(matches []rules.Match) map[string][]rules.Match {
	grouped := make(map[string][]rules.Match)
	for _, m := range matches {
		grouped[m.File] = append(grouped[m.File], m)
	}
	return grouped
}
