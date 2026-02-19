// Project:   macbash
// File:      internal/rules/types.go
// Purpose:   Rule type definitions
// Language:  Go
//
// License:   Apache-2.0
// Copyright: (c) 2025 HyperSec Pty Ltd

package rules

type Severity string

const (
	SeverityError   Severity = "error"
	SeverityWarning Severity = "warning"
	SeverityInfo    Severity = "info"
)

type FixType string

const (
	FixReplace   FixType = "replace"
	FixSuggest   FixType = "suggest"
	FixFunction  FixType = "function"
	FixTransform FixType = "transform"
)

type Rule struct {
	ID              string   `yaml:"id"`
	Name            string   `yaml:"name"`
	Description     string   `yaml:"description"`
	Severity        Severity `yaml:"severity"`
	Pattern         string   `yaml:"pattern"`
	NegativePattern string   `yaml:"negative_pattern,omitempty"`
	ShebangMatch    string   `yaml:"shebang_match,omitempty"`
	FixType         FixType  `yaml:"fix_type"`
	FixTemplate     string   `yaml:"fix_template,omitempty"`
	WhyUnfixable    string   `yaml:"why_unfixable,omitempty"`
	FixFunction     string   `yaml:"fix_function,omitempty"`
	Examples        Examples `yaml:"examples,omitempty"`
	Tags            []string `yaml:"tags,omitempty"`
	References      []string `yaml:"references,omitempty"`
}

type Examples struct {
	Bad  string `yaml:"bad,omitempty"`
	Good string `yaml:"good,omitempty"`
}

type RuleSet struct {
	Version string `yaml:"version"`
	Rules   []Rule `yaml:"rules"`
}

type Match struct {
	Rule       *Rule
	File       string
	Line       int
	Column     int
	Content    string
	MatchedStr string
	FixedStr   string
}
