// Project:   macbash
// File:      internal/cli/root.go
// Purpose:   Root CLI command definition
// Language:  Go
//
// License:   Apache-2.0
// Copyright: (c) 2025 HyperSec Pty Ltd

package cli

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/spf13/cobra"

	"github.com/hypersec-io/macbash/internal/fixer"
	"github.com/hypersec-io/macbash/internal/output"
	"github.com/hypersec-io/macbash/internal/rules"
	"github.com/hypersec-io/macbash/internal/scanner"
)

var (
	configFile  string
	fix         bool
	write       bool
	outputPath  string
	severity    string
	format      string
	showVersion bool
	dryRun      bool

	appVersion   string
	appCommit    string
	appBuildTime string
)

func newRootCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "macbash [files...]",
		Short: "Check bash scripts for macOS compatibility",
		Long: `macbash checks bash scripts for GNU/Linux-specific constructs that
won't work on macOS (BSD), and can optionally fix them to be portable.

Examples:
  # Check a script for issues
  macbash script.sh

  # Fix and write changes back to the same file (in-place)
  macbash -w script.sh

  # Fix a single file and write to a new location
  macbash -o fixed_script.sh script.sh

  # Fix multiple files and write to an output directory
  macbash -o ./fixed/ scripts/*.sh

  # Preview fixes without writing (dry-run)
  macbash -w --dry-run script.sh

  # Check multiple files
  macbash scripts/*.sh

  # Use custom rules file
  macbash --config rules.yaml script.sh

  # Output as JSON for CI integration
  macbash --format json script.sh

Output Modes:
  (default)     Check only, report issues to stdout
  -w, --write   Fix and overwrite original files in-place
  -o, --output  Fix and write to specified path:
                - If single input file: -o is the output file path
                - If multiple input files: -o is the output directory

Exit Codes:
  0 - No issues found (or all fixed with -w/-o)
  1 - Errors found (unfixable or check-only mode)
  2 - Only warnings found (with --severity=warning)`,
		RunE:          runCheck,
		SilenceUsage:  true,
		SilenceErrors: true,
	}

	cmd.Flags().StringVarP(&configFile, "config", "c", "", "Path to custom rules YAML file")
	cmd.Flags().BoolVarP(&write, "write", "w", false, "Fix and overwrite files in-place")
	cmd.Flags().StringVarP(&outputPath, "output", "o", "", "Output path (file for single input, directory for multiple)")
	cmd.Flags().BoolVar(&dryRun, "dry-run", false, "Preview fixes without writing (use with -w or -o)")
	cmd.Flags().StringVarP(&severity, "severity", "s", "warning", "Minimum severity to report: error, warning, info")
	cmd.Flags().StringVar(&format, "format", "text", "Output format: text, json")
	cmd.Flags().BoolVarP(&showVersion, "version", "v", false, "Show version information")

	cmd.Flags().BoolVarP(&fix, "fix", "f", false, "Deprecated: use -w or -o instead")
	_ = cmd.Flags().MarkDeprecated("fix", "use -w (--write) or -o (--output) instead")

	return cmd
}

func runCheck(cmd *cobra.Command, args []string) error {
	if showVersion {
		fmt.Printf("macbash %s\n", appVersion)
		fmt.Printf("  commit:  %s\n", appCommit)
		fmt.Printf("  built:   %s\n", appBuildTime)
		return nil
	}

	if len(args) == 0 {
		return fmt.Errorf("no files specified. Use 'macbash --help' for usage")
	}

	if fix {
		write = true
		fmt.Fprintln(os.Stderr, "Warning: --fix is deprecated, use -w (--write) instead")
	}

	if write && outputPath != "" {
		return fmt.Errorf("cannot use both -w (--write) and -o (--output)")
	}

	if outputPath != "" && len(args) > 1 {
		info, err := os.Stat(outputPath)
		if err == nil && !info.IsDir() {
			return fmt.Errorf("output path must be a directory when processing multiple files")
		}
	}

	var minSeverity rules.Severity
	switch severity {
	case "error":
		minSeverity = rules.SeverityError
	case "warning":
		minSeverity = rules.SeverityWarning
	case "info":
		minSeverity = rules.SeverityInfo
	default:
		return fmt.Errorf("invalid severity %q: must be error, warning, or info", severity)
	}

	if format != "text" && format != "json" {
		return fmt.Errorf("invalid format %q: must be text or json", format)
	}

	for _, file := range args {
		if _, err := os.Stat(file); os.IsNotExist(err) {
			return fmt.Errorf("file not found: %s", file)
		}
	}

	ruleSet, err := loadRules()
	if err != nil {
		return fmt.Errorf("loading rules: %w", err)
	}

	s, err := scanner.New(ruleSet)
	if err != nil {
		return fmt.Errorf("creating scanner: %w", err)
	}

	matches, err := s.ScanFiles(args)
	if err != nil {
		return fmt.Errorf("scanning files: %w", err)
	}

	matches = scanner.FilterBySeverity(matches, minSeverity)

	if write || outputPath != "" {
		return runFix(args, matches, minSeverity)
	}

	formatter := output.New(os.Stdout)

	switch format {
	case "json":
		if err := formatter.JSON(matches); err != nil {
			return fmt.Errorf("outputting JSON: %w", err)
		}
	default:
		formatter.Text(matches)
	}

	if scanner.HasErrors(matches) {
		os.Exit(1)
	}

	return nil
}

func runFix(inputFiles []string, matches []rules.Match, _ rules.Severity) error {
	f := fixer.New()

	var fixedCount, unfixedCount int

	for _, inputFile := range inputFiles {
		fileMatches := filterMatchesByFile(matches, inputFile)
		outPath := determineOutputPath(inputFile, inputFiles)
		result, err := f.FixFile(inputFile, fileMatches)
		if err != nil {
			return fmt.Errorf("fixing %s: %w", inputFile, err)
		}

		fixedCount += result.FixedCount
		unfixedCount += result.UnfixedCount

		if result.ValidationErr != nil {
			fmt.Fprintf(os.Stderr, "WARNING: Fix for %s would create invalid syntax: %v\n", inputFile, result.ValidationErr)
			fmt.Fprintf(os.Stderr, "  Skipping write for this file to preserve original\n")
			unfixedCount += result.FixedCount // Count as unfixed since we won't apply
			fixedCount -= result.FixedCount
			continue
		}

		if !dryRun {
			if err := writeOutput(outPath, result.Content); err != nil {
				return fmt.Errorf("writing %s: %w", outPath, err)
			}
			switch {
			case result.FixedCount > 0 && result.UnfixedCount > 0:
				fmt.Printf("Partially fixed %s -> %s (%d fixes applied, %d unfixable)\n",
					inputFile, outPath, result.FixedCount, result.UnfixedCount)
			case result.FixedCount > 0:
				fmt.Printf("Fixed %s -> %s (%d fixes applied)\n",
					inputFile, outPath, result.FixedCount)
			default:
				fmt.Printf("Copied %s -> %s (0 fixes applied, %d unfixable)\n",
					inputFile, outPath, result.UnfixedCount)
			}
		} else {
			fmt.Printf("[dry-run] Would fix %s -> %s (%d fixes, %d unfixable)\n",
				inputFile, outPath, result.FixedCount, result.UnfixedCount)
			if result.FixedCount > 0 {
				fmt.Println("--- Preview of changes ---")
				fmt.Println(result.Content)
				fmt.Println("--- End preview ---")
			}
		}
	}

	fmt.Printf("\nTotal: %d fixes applied, %d unfixable issues\n", fixedCount, unfixedCount)

	if unfixedCount > 0 {
		os.Exit(1)
	}

	return nil
}

func determineOutputPath(inputFile string, allInputFiles []string) string {
	if write {
		return inputFile
	}
	if outputPath != "" {
		if len(allInputFiles) == 1 {
			return outputPath
		}
		return filepath.Join(outputPath, filepath.Base(inputFile))
	}
	return inputFile
}

func filterMatchesByFile(matches []rules.Match, file string) []rules.Match {
	var filtered []rules.Match
	for _, m := range matches {
		if m.File == file {
			filtered = append(filtered, m)
		}
	}
	return filtered
}

func writeOutput(path, content string) error {
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return err
	}

	return os.WriteFile(path, []byte(content), 0o644)
}

func loadRules() (*rules.RuleSet, error) {
	builtin, err := rules.LoadBuiltin()
	if err != nil {
		return nil, fmt.Errorf("loading builtin rules: %w", err)
	}

	if configFile != "" {
		custom, err := rules.LoadFromFile(configFile)
		if err != nil {
			return nil, fmt.Errorf("loading custom rules: %w", err)
		}
		return rules.Merge(builtin, custom), nil
	}

	return builtin, nil
}

func Execute(version, commit, buildTime string) error {
	appVersion = version
	appCommit = commit
	appBuildTime = buildTime

	return newRootCmd().Execute()
}
