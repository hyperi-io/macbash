// Project:   macbash
// File:      internal/scanner/scanner_test.go
// Purpose:   Tests for the scanner package
// Language:  Go
//
// License:   Apache-2.0
// Copyright: (c) 2025 HyperSec Pty Ltd

package scanner

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/hypersec-io/macbash/internal/rules"
)

func TestNew(t *testing.T) {
	rs := &rules.RuleSet{
		Rules: []rules.Rule{
			{
				ID:       "test-rule",
				Name:     "Test Rule",
				Pattern:  `echo\s+-e`,
				Severity: rules.SeverityWarning,
			},
		},
	}

	s, err := New(rs)
	if err != nil {
		t.Fatalf("New() error = %v", err)
	}

	if s == nil {
		t.Fatal("New() returned nil scanner")
	}

	if len(s.compiled) != 1 {
		t.Errorf("expected 1 compiled rule, got %d", len(s.compiled))
	}
}

func TestNew_InvalidPattern(t *testing.T) {
	rs := &rules.RuleSet{
		Rules: []rules.Rule{
			{
				ID:       "bad-rule",
				Name:     "Bad Rule",
				Pattern:  `[invalid`,
				Severity: rules.SeverityWarning,
			},
		},
	}

	_, err := New(rs)
	if err == nil {
		t.Error("New() should fail with invalid pattern")
	}
}

func TestScanFile(t *testing.T) {
	// Create a temporary test file
	dir := t.TempDir()
	testFile := filepath.Join(dir, "test.sh")
	content := `#!/bin/bash
echo -e "hello\nworld"
grep -P '\d+' file.txt
`
	if err := os.WriteFile(testFile, []byte(content), 0o644); err != nil {
		t.Fatalf("failed to create test file: %v", err)
	}

	rs := &rules.RuleSet{
		Rules: []rules.Rule{
			{
				ID:       "echo-escape",
				Name:     "echo -e",
				Pattern:  `echo\s+-e\s`,
				Severity: rules.SeverityWarning,
			},
			{
				ID:       "grep-perl",
				Name:     "grep -P",
				Pattern:  `grep\s+-P\s`,
				Severity: rules.SeverityError,
			},
		},
	}

	s, err := New(rs)
	if err != nil {
		t.Fatalf("New() error = %v", err)
	}

	matches, err := s.ScanFile(testFile)
	if err != nil {
		t.Fatalf("ScanFile() error = %v", err)
	}

	if len(matches) != 2 {
		t.Errorf("expected 2 matches, got %d", len(matches))
	}

	// Check first match
	if matches[0].Rule.ID != "echo-escape" {
		t.Errorf("expected echo-escape rule, got %s", matches[0].Rule.ID)
	}
	if matches[0].Line != 2 {
		t.Errorf("expected line 2, got %d", matches[0].Line)
	}

	// Check second match
	if matches[1].Rule.ID != "grep-perl" {
		t.Errorf("expected grep-perl rule, got %s", matches[1].Rule.ID)
	}
	if matches[1].Line != 3 {
		t.Errorf("expected line 3, got %d", matches[1].Line)
	}
}

func TestScanFile_NegativePattern(t *testing.T) {
	dir := t.TempDir()
	testFile := filepath.Join(dir, "test.sh")
	content := `#!/bin/bash
# Check if realpath exists before using
command -v realpath && realpath /some/path
realpath /another/path
`
	if err := os.WriteFile(testFile, []byte(content), 0o644); err != nil {
		t.Fatalf("failed to create test file: %v", err)
	}

	rs := &rules.RuleSet{
		Rules: []rules.Rule{
			{
				ID:              "realpath-unchecked",
				Name:            "realpath without check",
				Pattern:         `\brealpath\s+`,
				NegativePattern: `command\s+-v\s+realpath`,
				Severity:        rules.SeverityWarning,
			},
		},
	}

	s, err := New(rs)
	if err != nil {
		t.Fatalf("New() error = %v", err)
	}

	matches, err := s.ScanFile(testFile)
	if err != nil {
		t.Fatalf("ScanFile() error = %v", err)
	}

	// Should only match line 4, not line 3 (which has the negative pattern)
	if len(matches) != 1 {
		t.Errorf("expected 1 match, got %d", len(matches))
	}
	if len(matches) > 0 && matches[0].Line != 4 {
		t.Errorf("expected match on line 4, got line %d", matches[0].Line)
	}
}

func TestFilterBySeverity(t *testing.T) {
	matches := []rules.Match{
		{Rule: &rules.Rule{Severity: rules.SeverityInfo}},
		{Rule: &rules.Rule{Severity: rules.SeverityWarning}},
		{Rule: &rules.Rule{Severity: rules.SeverityError}},
		{Rule: &rules.Rule{Severity: rules.SeverityInfo}},
	}

	tests := []struct {
		name        string
		minSeverity rules.Severity
		wantCount   int
	}{
		{"info", rules.SeverityInfo, 4},
		{"warning", rules.SeverityWarning, 2},
		{"error", rules.SeverityError, 1},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			filtered := FilterBySeverity(matches, tt.minSeverity)
			if len(filtered) != tt.wantCount {
				t.Errorf("FilterBySeverity(%s) = %d matches, want %d", tt.name, len(filtered), tt.wantCount)
			}
		})
	}
}

func TestHasErrors(t *testing.T) {
	tests := []struct {
		name    string
		matches []rules.Match
		want    bool
	}{
		{
			name:    "no matches",
			matches: []rules.Match{},
			want:    false,
		},
		{
			name: "only warnings",
			matches: []rules.Match{
				{Rule: &rules.Rule{Severity: rules.SeverityWarning}},
			},
			want: false,
		},
		{
			name: "has error",
			matches: []rules.Match{
				{Rule: &rules.Rule{Severity: rules.SeverityWarning}},
				{Rule: &rules.Rule{Severity: rules.SeverityError}},
			},
			want: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := HasErrors(tt.matches); got != tt.want {
				t.Errorf("HasErrors() = %v, want %v", got, tt.want)
			}
		})
	}
}
