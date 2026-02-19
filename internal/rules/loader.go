// Project:   macbash
// File:      internal/rules/loader.go
// Purpose:   Load rules from YAML files
// Language:  Go
//
// License:   Apache-2.0
// Copyright: (c) 2025 HyperSec Pty Ltd

package rules

import (
	"embed"
	"fmt"
	"os"
	"regexp"

	"gopkg.in/yaml.v3"
)

//go:embed builtin/*.yaml
var builtinRules embed.FS

func LoadFromFile(path string) (*RuleSet, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("reading rules file %s: %w", path, err)
	}

	return parseRuleSet(data, path)
}

func LoadBuiltin() (*RuleSet, error) {
	entries, err := builtinRules.ReadDir("builtin")
	if err != nil {
		return nil, fmt.Errorf("reading builtin rules directory: %w", err)
	}

	combined := &RuleSet{
		Version: "1.0",
		Rules:   []Rule{},
	}

	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}

		data, err := builtinRules.ReadFile("builtin/" + entry.Name())
		if err != nil {
			return nil, fmt.Errorf("reading builtin rule %s: %w", entry.Name(), err)
		}

		rs, err := parseRuleSet(data, entry.Name())
		if err != nil {
			return nil, fmt.Errorf("parsing builtin rule %s: %w", entry.Name(), err)
		}

		combined.Rules = append(combined.Rules, rs.Rules...)
	}

	return combined, nil
}

func parseRuleSet(data []byte, source string) (*RuleSet, error) {
	var rs RuleSet
	if err := yaml.Unmarshal(data, &rs); err != nil {
		return nil, fmt.Errorf("parsing YAML from %s: %w", source, err)
	}

	// Validate rules
	for i := range rs.Rules {
		if err := validateRule(&rs.Rules[i]); err != nil {
			return nil, fmt.Errorf("invalid rule %d (%s) in %s: %w", i, rs.Rules[i].ID, source, err)
		}
	}

	return &rs, nil
}

func validateRule(r *Rule) error {
	if r.ID == "" {
		return fmt.Errorf("rule missing ID")
	}

	if r.Name == "" {
		return fmt.Errorf("rule %s missing name", r.ID)
	}

	if r.Pattern == "" {
		return fmt.Errorf("rule %s missing pattern", r.ID)
	}

	// Validate pattern compiles
	if _, err := regexp.Compile(r.Pattern); err != nil {
		return fmt.Errorf("rule %s has invalid pattern: %w", r.ID, err)
	}

	// Validate negative pattern if present
	if r.NegativePattern != "" {
		if _, err := regexp.Compile(r.NegativePattern); err != nil {
			return fmt.Errorf("rule %s has invalid negative_pattern: %w", r.ID, err)
		}
	}

	// Validate shebang match pattern if present
	if r.ShebangMatch != "" {
		if _, err := regexp.Compile(r.ShebangMatch); err != nil {
			return fmt.Errorf("rule %s has invalid shebang_match: %w", r.ID, err)
		}
	}

	// Validate severity
	switch r.Severity {
	case SeverityError, SeverityWarning, SeverityInfo:
		// Valid
	case "":
		r.Severity = SeverityWarning // Default
	default:
		return fmt.Errorf("rule %s has invalid severity %q", r.ID, r.Severity)
	}

	// Validate fix type
	switch r.FixType {
	case FixReplace, FixSuggest, FixFunction, FixTransform, "":
		// Valid (empty defaults to suggest)
		if r.FixType == "" {
			r.FixType = FixSuggest
		}
	default:
		return fmt.Errorf("rule %s has invalid fix_type %q", r.ID, r.FixType)
	}

	return nil
}

func Merge(sets ...*RuleSet) *RuleSet {
	seen := make(map[string]int)
	combined := &RuleSet{
		Version: "1.0",
		Rules:   []Rule{},
	}

	for _, rs := range sets {
		if rs == nil {
			continue
		}
		for i := range rs.Rules {
			rule := &rs.Rules[i]
			if idx, exists := seen[rule.ID]; exists {
				// Override existing rule
				combined.Rules[idx] = *rule
			} else {
				seen[rule.ID] = len(combined.Rules)
				combined.Rules = append(combined.Rules, *rule)
			}
		}
	}

	return combined
}
