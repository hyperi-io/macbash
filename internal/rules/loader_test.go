// Project:   macbash
// File:      internal/rules/loader_test.go
// Purpose:   Tests for the rules loader
// Language:  Go
//
// License:   Apache-2.0
// Copyright: (c) 2025 HyperSec Pty Ltd

package rules

import (
	"os"
	"path/filepath"
	"testing"
)

func TestLoadBuiltin(t *testing.T) {
	rs, err := LoadBuiltin()
	if err != nil {
		t.Fatalf("LoadBuiltin() error = %v", err)
	}

	if rs == nil {
		t.Fatal("LoadBuiltin() returned nil")
	}

	if len(rs.Rules) == 0 {
		t.Error("LoadBuiltin() returned empty ruleset")
	}

	// Check some expected rules exist
	expectedIDs := []string{
		"sed-inplace-no-backup",
		"grep-perl-regex",
		"readlink-canonicalize",
		"date-d-epoch",
		"bash4-associative-array",
	}

	ruleIDs := make(map[string]bool)
	for _, r := range rs.Rules {
		ruleIDs[r.ID] = true
	}

	for _, id := range expectedIDs {
		if !ruleIDs[id] {
			t.Errorf("expected rule %q not found in builtin rules", id)
		}
	}
}

func TestLoadFromFile(t *testing.T) {
	dir := t.TempDir()
	testFile := filepath.Join(dir, "test-rules.yaml")

	content := `version: "1.0"
rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    severity: warning
    pattern: 'test\s+pattern'
    fix_type: suggest
    fix_template: fixed pattern
`
	if err := os.WriteFile(testFile, []byte(content), 0o644); err != nil {
		t.Fatalf("failed to create test file: %v", err)
	}

	rs, err := LoadFromFile(testFile)
	if err != nil {
		t.Fatalf("LoadFromFile() error = %v", err)
	}

	if len(rs.Rules) != 1 {
		t.Errorf("expected 1 rule, got %d", len(rs.Rules))
	}

	r := rs.Rules[0]
	if r.ID != "test-rule" {
		t.Errorf("expected ID 'test-rule', got %q", r.ID)
	}
	if r.Severity != SeverityWarning {
		t.Errorf("expected severity 'warning', got %q", r.Severity)
	}
}

func TestLoadFromFile_InvalidYAML(t *testing.T) {
	dir := t.TempDir()
	testFile := filepath.Join(dir, "invalid.yaml")

	content := `this is not: valid: yaml: syntax`
	if err := os.WriteFile(testFile, []byte(content), 0o644); err != nil {
		t.Fatalf("failed to create test file: %v", err)
	}

	_, err := LoadFromFile(testFile)
	if err == nil {
		t.Error("LoadFromFile() should fail with invalid YAML")
	}
}

func TestLoadFromFile_InvalidPattern(t *testing.T) {
	dir := t.TempDir()
	testFile := filepath.Join(dir, "bad-pattern.yaml")

	content := `version: "1.0"
rules:
  - id: bad-rule
    name: Bad Rule
    pattern: '[invalid'
    severity: error
`
	if err := os.WriteFile(testFile, []byte(content), 0o644); err != nil {
		t.Fatalf("failed to create test file: %v", err)
	}

	_, err := LoadFromFile(testFile)
	if err == nil {
		t.Error("LoadFromFile() should fail with invalid regex pattern")
	}
}

func TestValidateRule(t *testing.T) {
	tests := []struct {
		name    string
		rule    Rule
		wantErr bool
	}{
		{
			name: "valid rule",
			rule: Rule{
				ID:       "test",
				Name:     "Test",
				Pattern:  `test`,
				Severity: SeverityWarning,
			},
			wantErr: false,
		},
		{
			name: "missing ID",
			rule: Rule{
				Name:    "Test",
				Pattern: `test`,
			},
			wantErr: true,
		},
		{
			name: "missing name",
			rule: Rule{
				ID:      "test",
				Pattern: `test`,
			},
			wantErr: true,
		},
		{
			name: "missing pattern",
			rule: Rule{
				ID:   "test",
				Name: "Test",
			},
			wantErr: true,
		},
		{
			name: "invalid pattern",
			rule: Rule{
				ID:      "test",
				Name:    "Test",
				Pattern: `[invalid`,
			},
			wantErr: true,
		},
		{
			name: "invalid severity",
			rule: Rule{
				ID:       "test",
				Name:     "Test",
				Pattern:  `test`,
				Severity: "invalid",
			},
			wantErr: true,
		},
		{
			name: "default severity",
			rule: Rule{
				ID:      "test",
				Name:    "Test",
				Pattern: `test`,
			},
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := validateRule(&tt.rule)
			if (err != nil) != tt.wantErr {
				t.Errorf("validateRule() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestMerge(t *testing.T) {
	rs1 := &RuleSet{
		Rules: []Rule{
			{ID: "rule1", Name: "Rule 1", Pattern: "pattern1"},
			{ID: "rule2", Name: "Rule 2", Pattern: "pattern2"},
		},
	}

	rs2 := &RuleSet{
		Rules: []Rule{
			{ID: "rule2", Name: "Rule 2 Override", Pattern: "pattern2-new"},
			{ID: "rule3", Name: "Rule 3", Pattern: "pattern3"},
		},
	}

	merged := Merge(rs1, rs2)

	if len(merged.Rules) != 3 {
		t.Errorf("expected 3 rules, got %d", len(merged.Rules))
	}

	// Check rule2 was overridden
	for _, r := range merged.Rules {
		if r.ID == "rule2" {
			if r.Name != "Rule 2 Override" {
				t.Errorf("rule2 should be overridden, got name %q", r.Name)
			}
		}
	}
}

func TestMerge_NilSets(t *testing.T) {
	rs1 := &RuleSet{
		Rules: []Rule{
			{ID: "rule1", Name: "Rule 1", Pattern: "pattern1"},
		},
	}

	merged := Merge(nil, rs1, nil)

	if len(merged.Rules) != 1 {
		t.Errorf("expected 1 rule, got %d", len(merged.Rules))
	}
}
