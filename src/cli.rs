//  Project:   macbash
//  File:      src/cli.rs
//  Purpose:   CLI argument parsing and dispatch
//  Language:  Rust
//
//  License:   Apache-2.0
//  Copyright: (c) 2025-2026 HYPERI PTY LIMITED

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;

use crate::fixer::{FixOutcome, Fixer, validate_bash_syntax};
use crate::output::Formatter;
use crate::rules::{MatchHit, RuleSet, Severity, load_builtin, load_from_file, merge_some};
use crate::scanner::{Scanner, filter_by_severity, has_errors};

const LONG_ABOUT: &str = "macbash checks bash scripts for GNU/Linux-specific constructs that
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
  1 - Errors found (unfixable or check-only mode)";

#[derive(Parser, Debug)]
#[command(
    name = "macbash",
    bin_name = "macbash",
    about = "Check bash scripts for macOS compatibility",
    long_about = LONG_ABOUT,
    disable_version_flag = true,
    arg_required_else_help = false,
)]
struct Cli {
    /// Path to custom rules YAML file
    #[arg(short = 'c', long = "config")]
    config: Option<PathBuf>,

    /// Fix and overwrite files in-place
    #[arg(short = 'w', long = "write")]
    write: bool,

    /// Output path (file for single input, directory for multiple)
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    /// Preview fixes without writing (use with -w or -o)
    #[arg(long = "dry-run")]
    dry_run: bool,

    /// Minimum severity to report: error, warning, info
    #[arg(short = 's', long = "severity", default_value = "warning")]
    severity: String,

    /// Output format: text, json
    #[arg(long = "format", default_value = "text")]
    format: String,

    /// Show version information
    #[arg(short = 'v', long = "version")]
    show_version: bool,

    /// Deprecated: use -w or -o instead
    #[arg(short = 'f', long = "fix", hide = true)]
    deprecated_fix: bool,

    files: Vec<PathBuf>,
}

pub fn run() -> ExitCode {
    let cli = Cli::parse();
    match run_inner(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e:#}");
            ExitCode::from(1)
        }
    }
}

fn run_inner(mut cli: Cli) -> Result<ExitCode> {
    if cli.show_version {
        println!("macbash {}", env!("CARGO_PKG_VERSION"));
        println!("  commit:  {}", env!("VERGEN_GIT_SHA"));
        println!("  built:   {}", env!("VERGEN_BUILD_TIMESTAMP"));
        return Ok(ExitCode::SUCCESS);
    }
    if cli.files.is_empty() {
        anyhow::bail!("no files specified. Use 'macbash --help' for usage");
    }
    if cli.deprecated_fix {
        cli.write = true;
        eprintln!("Warning: --fix is deprecated, use -w (--write) instead");
    }
    if cli.write && cli.output.is_some() {
        anyhow::bail!("cannot use both -w (--write) and -o (--output)");
    }
    if let Some(out) = &cli.output
        && cli.files.len() > 1
        && let Ok(meta) = std::fs::metadata(out)
        && !meta.is_dir()
    {
        anyhow::bail!("output path must be a directory when processing multiple files");
    }
    let min_severity = match cli.severity.as_str() {
        "error" => Severity::Error,
        "warning" => Severity::Warning,
        "info" => Severity::Info,
        other => anyhow::bail!("invalid severity {other:?}: must be error, warning, or info"),
    };
    if cli.format != "text" && cli.format != "json" {
        anyhow::bail!("invalid format {:?}: must be text or json", cli.format);
    }
    for f in &cli.files {
        if !f.exists() {
            anyhow::bail!("file not found: {}", f.display());
        }
    }

    let rules = load_rules(cli.config.as_deref()).context("loading rules")?;
    let scanner = Scanner::new(&rules).context("compiling rule patterns")?;
    let mut matches = scanner
        .scan_files(cli.files.iter().map(PathBuf::as_path))
        .context("scanning files")?;
    matches = filter_by_severity(matches, min_severity, &rules);

    if cli.write || cli.output.is_some() {
        return run_fix(&cli, &matches, &rules);
    }

    let stdout = std::io::stdout();
    let use_colour = stdout.is_terminal();
    let mut handle = stdout.lock();
    let mut formatter = Formatter::new(&mut handle, use_colour, &rules);
    match cli.format.as_str() {
        "json" => formatter.json(&matches)?,
        _ => formatter.text(&matches)?,
    }
    if has_errors(&matches, &rules) {
        return Ok(ExitCode::from(1));
    }
    Ok(ExitCode::SUCCESS)
}

fn run_fix(cli: &Cli, matches: &[MatchHit], rules: &RuleSet) -> Result<ExitCode> {
    eprintln!(
        "EXPERIMENTAL: macbash fix mode is experimental — review output and diff \
         before committing rewritten scripts."
    );
    let mut fixer = Fixer::new(rules);
    let mut fixed_total = 0_usize;
    let mut unfixed_total = 0_usize;
    for input in &cli.files {
        let file_matches: Vec<MatchHit> = matches
            .iter()
            .filter(|m| Path::new(&m.file) == input.as_path())
            .cloned()
            .collect();
        let outcome: FixOutcome = fixer.fix_file(input, &file_matches)?;
        let out_path = determine_output_path(input, &cli.files, cli);
        let validation_err = if outcome.fixed_count > 0 {
            validate_bash_syntax(&outcome.content).err()
        } else {
            None
        };
        if let Some(err) = validation_err {
            eprintln!(
                "WARNING: Fix for {} would create invalid syntax: {err}",
                input.display()
            );
            eprintln!("  Skipping write for this file to preserve original");
            unfixed_total += outcome.fixed_count + outcome.unfixed_count;
            continue;
        }
        fixed_total += outcome.fixed_count;
        unfixed_total += outcome.unfixed_count;
        if cli.dry_run {
            println!(
                "[dry-run] Would fix {} -> {} ({} fixes, {} unfixable)",
                input.display(),
                out_path.display(),
                outcome.fixed_count,
                outcome.unfixed_count
            );
            if outcome.fixed_count > 0 {
                println!("--- Preview of changes ---");
                println!("{}", outcome.content);
                println!("--- End preview ---");
            }
            continue;
        }
        write_output(&out_path, &outcome.content)
            .with_context(|| format!("writing {}", out_path.display()))?;
        match (outcome.fixed_count, outcome.unfixed_count) {
            (f, u) if f > 0 && u > 0 => println!(
                "Partially fixed {} -> {} ({f} fixes applied, {u} unfixable)",
                input.display(),
                out_path.display()
            ),
            (f, _) if f > 0 => println!(
                "Fixed {} -> {} ({f} fixes applied)",
                input.display(),
                out_path.display()
            ),
            (_, u) => println!(
                "Copied {} -> {} (0 fixes applied, {u} unfixable)",
                input.display(),
                out_path.display()
            ),
        }
    }
    println!("\nTotal: {fixed_total} fixes applied, {unfixed_total} unfixable issues");
    if unfixed_total > 0 {
        return Ok(ExitCode::from(1));
    }
    Ok(ExitCode::SUCCESS)
}

fn determine_output_path(input: &Path, all: &[PathBuf], cli: &Cli) -> PathBuf {
    if cli.write {
        return input.to_path_buf();
    }
    if let Some(out) = &cli.output {
        if all.len() == 1 {
            return out.clone();
        }
        let name = input.file_name().unwrap_or_default();
        return out.join(name);
    }
    input.to_path_buf()
}

fn write_output(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)
}

fn load_rules(custom: Option<&Path>) -> Result<RuleSet> {
    let builtin = load_builtin().context("loading builtin rules")?;
    if let Some(path) = custom {
        let extra = load_from_file(path).context("loading custom rules")?;
        return Ok(merge_some(&[&builtin, &extra]));
    }
    Ok(builtin)
}
