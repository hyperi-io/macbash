// Project:   macbash
// File:      internal/rules/pattern_test.go
// Purpose:   AB testing harness for rule patterns - tests both matches and non-matches
// Language:  Go
//
// License:   Apache-2.0
// Copyright: (c) 2025-2026 HYPERI PTY LIMITED

package rules

import (
	"fmt"
	"regexp"
	"testing"
)

// TestRulePatterns_AB runs every rule's test_cases through the compiled regex,
// asserting that should_match lines match and should_not_match lines don't.
// This catches both false negatives (missed detections) and false positives
// (incorrect detections).
//
// IMPORTANT: These tests operate at the REGEX level only. The scanner provides
// additional filtering (comment skipping, heredoc skipping, shebang matching)
// that is NOT tested here. Test vectors in should_not_match should only include
// lines that are NOT comments and should genuinely not trigger the rule at the
// regex level. Comment/heredoc/shebang filtering is tested in scanner_test.go.
func TestRulePatterns_AB(t *testing.T) {
	rs, err := LoadBuiltin()
	if err != nil {
		t.Fatalf("LoadBuiltin() error = %v", err)
	}

	rulesWithTests := 0
	totalShouldMatch := 0
	totalShouldNotMatch := 0

	for _, r := range rs.Rules {
		if len(r.TestCases.ShouldMatch) == 0 && len(r.TestCases.ShouldNotMatch) == 0 {
			continue
		}
		rulesWithTests++

		t.Run(r.ID, func(t *testing.T) {
			pattern, err := regexp.Compile(r.Pattern)
			if err != nil {
				t.Fatalf("failed to compile pattern %q: %v", r.Pattern, err)
			}

			var negative *regexp.Regexp
			if r.NegativePattern != "" {
				negative, err = regexp.Compile(r.NegativePattern)
				if err != nil {
					t.Fatalf("failed to compile negative pattern %q: %v", r.NegativePattern, err)
				}
			}

			// Test true positives: these lines MUST match
			for i, line := range r.TestCases.ShouldMatch {
				totalShouldMatch++
				t.Run(fmt.Sprintf("should_match[%d]", i), func(t *testing.T) {
					matched := pattern.MatchString(line)
					if !matched {
						t.Errorf("FALSE NEGATIVE: pattern %q did not match line:\n  %q", r.Pattern, line)
						return
					}
					// If there's a negative pattern, it should NOT suppress this match
					if negative != nil && negative.MatchString(line) {
						t.Errorf("SUPPRESSED: negative pattern %q unexpectedly matched line:\n  %q\n  This line is in should_match but would be suppressed by the negative pattern", r.NegativePattern, line)
					}
				})
			}

			// Test true negatives: these lines must NOT match (at regex level)
			for i, line := range r.TestCases.ShouldNotMatch {
				totalShouldNotMatch++
				t.Run(fmt.Sprintf("should_not_match[%d]", i), func(t *testing.T) {
					matched := pattern.MatchString(line)
					if !matched {
						return // Good: pattern correctly doesn't match
					}
					// Pattern matched - check if negative pattern saves us
					if negative != nil && negative.MatchString(line) {
						return // Good: negative pattern correctly suppresses
					}
					t.Errorf("FALSE POSITIVE: pattern %q incorrectly matched line:\n  %q\n  matched: %q", r.Pattern, line, pattern.FindString(line))
				})
			}
		})
	}

	t.Logf("Tested %d rules with test vectors (%d should_match, %d should_not_match)",
		rulesWithTests, totalShouldMatch, totalShouldNotMatch)
}

// TestAllRulesHaveTestCases ensures every builtin rule has test vectors.
// This prevents new rules from being added without proper testing.
func TestAllRulesHaveTestCases(t *testing.T) {
	rs, err := LoadBuiltin()
	if err != nil {
		t.Fatalf("LoadBuiltin() error = %v", err)
	}

	var missing []string
	for _, r := range rs.Rules {
		if len(r.TestCases.ShouldMatch) == 0 {
			missing = append(missing, fmt.Sprintf("%s (no should_match)", r.ID))
		}
		if len(r.TestCases.ShouldNotMatch) == 0 {
			missing = append(missing, fmt.Sprintf("%s (no should_not_match)", r.ID))
		}
	}

	if len(missing) > 0 {
		t.Errorf("%d rules missing test vectors:", len(missing))
		for _, m := range missing {
			t.Logf("  - %s", m)
		}
	}
}
