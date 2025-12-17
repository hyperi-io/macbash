// Project:   macbash
// File:      internal/fixer/fixer.go
// Purpose:   Apply fixes to bash scripts
// Language:  Go
//
// License:   Apache-2.0
// Copyright: (c) 2025 HyperSec Pty Ltd

package fixer

import (
	"bufio"
	"fmt"
	"os"
	"regexp"
	"sort"
	"strings"

	"mvdan.cc/sh/v3/syntax"

	"github.com/hypersec-io/macbash/internal/rules"
)

type Fixer struct {
	compiled map[string]*regexp.Regexp
}

type Result struct {
	Content       string
	FixedCount    int
	UnfixedCount  int
	Fixes         []AppliedFix
	ValidationErr error
}

type AppliedFix struct {
	Line       int
	RuleID     string
	Original   string
	Fixed      string
	WasSuggest bool
}

func New() *Fixer {
	return &Fixer{
		compiled: make(map[string]*regexp.Regexp),
	}
}

func (f *Fixer) FixFile(path string, matches []rules.Match) (*Result, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	matchesByLine := make(map[int][]rules.Match)
	for _, m := range matches {
		matchesByLine[m.Line] = append(matchesByLine[m.Line], m)
	}

	// Rightmost first to preserve column positions
	for line := range matchesByLine {
		sort.Slice(matchesByLine[line], func(i, j int) bool {
			return matchesByLine[line][i].Column > matchesByLine[line][j].Column
		})
	}

	var result Result
	var outputLines []string
	scanner := bufio.NewScanner(file)
	lineNum := 0

	for scanner.Scan() {
		lineNum++
		line := scanner.Text()

		if lineMatches, ok := matchesByLine[lineNum]; ok {
			fixedLine, fixes, unfixed := f.fixLine(line, lineMatches)
			outputLines = append(outputLines, fixedLine)
			result.Fixes = append(result.Fixes, fixes...)
			result.FixedCount += len(fixes)
			result.UnfixedCount += unfixed
		} else {
			outputLines = append(outputLines, line)
		}
	}

	if err := scanner.Err(); err != nil {
		return nil, err
	}

	result.Content = strings.Join(outputLines, "\n") + "\n"

	if result.FixedCount > 0 {
		if err := validateBashSyntax(result.Content); err != nil {
			result.ValidationErr = err
		}
	}

	return &result, nil
}

func (f *Fixer) fixLine(line string, matches []rules.Match) (fixedLine string, fixes []AppliedFix, unfixed int) {
	fixedLine = line

	for _, m := range matches {
		var newResult string
		var applied bool

		switch m.Rule.FixType {
		case rules.FixReplace:
			re, err := f.getCompiledPattern(m.Rule)
			if err != nil {
				unfixed++
				continue
			}

			newResult = re.ReplaceAllString(fixedLine, m.Rule.FixTemplate)
			applied = newResult != fixedLine

		case rules.FixTransform:
			newResult, applied = f.applyTransform(fixedLine, &m)

		default:
			unfixed++
			continue
		}

		if applied {
			fixes = append(fixes, AppliedFix{
				Line:     m.Line,
				RuleID:   m.Rule.ID,
				Original: fixedLine,
				Fixed:    newResult,
			})
			fixedLine = newResult
		} else {
			unfixed++
		}
	}

	return fixedLine, fixes, unfixed
}

func (f *Fixer) getCompiledPattern(rule *rules.Rule) (*regexp.Regexp, error) {
	if re, ok := f.compiled[rule.ID]; ok {
		return re, nil
	}

	re, err := regexp.Compile(rule.Pattern)
	if err != nil {
		return nil, err
	}

	f.compiled[rule.ID] = re
	return re, nil
}

func (f *Fixer) applyTransform(line string, m *rules.Match) (string, bool) {
	switch m.Rule.ID {
	case "grep-perl-regex", "grep-only-matching-P":
		return f.transformGrepPtoE(line, m)
	default:
		return line, false
	}
}

// Converts simple grep -P to grep -E. Skips patterns with \K, lookbehinds, etc.
func (f *Fixer) transformGrepPtoE(line string, _ *rules.Match) (string, bool) {
	grepReSingle := regexp.MustCompile(`(grep\s+)(-[a-zA-Z]*P[a-zA-Z]*)(\s+)'([^']*)'`)
	grepReDouble := regexp.MustCompile(`(grep\s+)(-[a-zA-Z]*P[a-zA-Z]*)(\s+)"([^"]*)"`)

	var matches []string
	var quote string
	var grepRe *regexp.Regexp

	matches = grepReSingle.FindStringSubmatch(line)
	if matches != nil {
		quote = "'"
		grepRe = grepReSingle
	} else {
		matches = grepReDouble.FindStringSubmatch(line)
		if matches != nil {
			quote = "\""
			grepRe = grepReDouble
		}
	}

	if matches == nil {
		return line, false
	}

	prefix := matches[1]
	flags := matches[2]
	space := matches[3]
	pattern := matches[4]

	if HasUnfixablePCRE(pattern) {
		return line, false
	}

	erePattern := pcreToERE(pattern)
	newFlags := strings.Replace(flags, "P", "E", 1)
	newGrep := prefix + newFlags + space + quote + erePattern + quote
	result := grepRe.ReplaceAllString(line, newGrep)

	return result, result != line
}

func ExtractGrepPattern(line string) string {
	grepReSingle := regexp.MustCompile(`grep\s+-[a-zA-Z]*P[a-zA-Z]*\s+'([^']*)'`)
	grepReDouble := regexp.MustCompile(`grep\s+-[a-zA-Z]*P[a-zA-Z]*\s+"([^"]*)"`)

	if matches := grepReSingle.FindStringSubmatch(line); matches != nil {
		return matches[1]
	}
	if matches := grepReDouble.FindStringSubmatch(line); matches != nil {
		return matches[1]
	}
	return ""
}

func CanTransformGrepP(line string) bool {
	pattern := ExtractGrepPattern(line)
	if pattern == "" {
		return false
	}
	return !HasUnfixablePCRE(pattern)
}

// PCRE features with no ERE equivalent
func HasUnfixablePCRE(pattern string) bool {
	unfixablePatterns := []string{
		`\K`, `(?=`, `(?!`, `(?<=`, `(?<!`, `(?:`, `(?P<`,
		`\b`, `\B`, `(?i)`, `(?m)`, `(?s)`,
	}

	for _, p := range unfixablePatterns {
		if strings.Contains(pattern, p) {
			return true
		}
	}

	return false
}

func pcreToERE(pattern string) string {
	replacements := []struct {
		pcre string
		ere  string
	}{
		{`\d`, `[0-9]`},
		{`\D`, `[^0-9]`},
		{`\w`, `[[:alnum:]_]`},
		{`\W`, `[^[:alnum:]_]`},
		{`\s`, `[[:space:]]`},
		{`\S`, `[^[:space:]]`},
	}

	result := pattern
	for _, r := range replacements {
		result = strings.ReplaceAll(result, r.pcre, r.ere)
	}

	return result
}

func validateBashSyntax(content string) error {
	parser := syntax.NewParser(
		syntax.Variant(syntax.LangBash),
		syntax.KeepComments(true),
	)

	_, err := parser.Parse(strings.NewReader(content), "")
	if err != nil {
		return fmt.Errorf("bash syntax error: %w", err)
	}
	return nil
}
