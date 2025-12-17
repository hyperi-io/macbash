#!/usr/bin/env bash
# Project:   macbash
# File:      scripts/test-local.sh
# Purpose:   Local development test loop - catches most issues before CI
# Language:  Bash
#
# License:   Apache-2.0
# Copyright: (c) 2025 HyperSec Pty Ltd
#
# Usage:
#   ./scripts/test-local.sh           # Full test suite
#   ./scripts/test-local.sh --quick   # Skip Docker tests (faster)
#   ./scripts/test-local.sh --fix     # Test fix mode against corpus

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
TESTS_PASSED=0
TESTS_FAILED=0

# Options
QUICK_MODE=false
TEST_FIX=false

for arg in "$@"; do
    case $arg in
        --quick) QUICK_MODE=true ;;
        --fix) TEST_FIX=true ;;
        --help|-h)
            echo "Usage: $0 [--quick] [--fix]"
            echo "  --quick  Skip Docker-based bash 3.2 tests"
            echo "  --fix    Test fix mode against corpus"
            exit 0
            ;;
    esac
done

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_pass() { echo -e "${GREEN}[PASS]${NC} $1"; TESTS_PASSED=$((TESTS_PASSED + 1)); }
log_fail() { echo -e "${RED}[FAIL]${NC} $1"; TESTS_FAILED=$((TESTS_FAILED + 1)); }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }

# =============================================================================
# Step 1: Build
# =============================================================================
log_info "Building macbash..."
if go build -o macbash ./cmd/macbash; then
    log_pass "Build succeeded"
else
    log_fail "Build failed"
    exit 1
fi

# =============================================================================
# Step 2: Go Tests
# =============================================================================
log_info "Running Go unit tests..."
if go test -race ./... > /tmp/macbash-test.log 2>&1; then
    log_pass "Go tests passed"
else
    log_fail "Go tests failed"
    cat /tmp/macbash-test.log
    exit 1
fi

# =============================================================================
# Step 3: Basic CLI Tests
# =============================================================================
log_info "Testing CLI basics..."

# Version flag
if ./macbash --version | grep -q "macbash"; then
    log_pass "Version flag works"
else
    log_fail "Version flag broken"
fi

# Help flag
if ./macbash --help 2>&1 | grep -qi "checks bash scripts"; then
    log_pass "Help flag works"
else
    log_fail "Help flag broken"
fi

# Check sample file (expect exit code 1 due to errors found)
OUTPUT=$(./macbash testdata/sample_linux.sh 2>&1 || true)
if echo "$OUTPUT" | grep -qi "error"; then
    log_pass "Detects issues in sample file"
else
    log_fail "Failed to detect issues in sample file"
fi

# JSON output (expect exit code 1 due to errors found)
JSON_OUTPUT=$(./macbash --format json testdata/sample_linux.sh 2>&1 || true)
if echo "$JSON_OUTPUT" | grep -q '"total_issues"'; then
    log_pass "JSON output works"
else
    log_fail "JSON output broken"
fi

# =============================================================================
# Step 4: Test Synthetic Scripts (fixer stress tests)
# =============================================================================
if [[ -d "testdata/scripts" ]] && [[ "$(ls -A testdata/scripts/*.sh 2>/dev/null)" ]]; then
    log_info "Testing synthetic scripts..."

    SYNTAX_ERRORS=0
    for script in testdata/scripts/*.sh; do
        # First verify original is valid
        if ! bash -n "$script" 2>/dev/null; then
            log_fail "Original script has syntax errors: $script"
            ((SYNTAX_ERRORS++)) || true
            continue
        fi

        # Fix the script
        FIXED_SCRIPT="/tmp/macbash_test_fixed_$(basename "$script")"
        if ./macbash -o "$FIXED_SCRIPT" "$script" 2>/dev/null; then
            : # OK - all issues were fixable
        else
            : # Expected - some issues are unfixable
        fi

        # Verify fixed script is still valid bash
        if [[ -f "$FIXED_SCRIPT" ]]; then
            if ! bash -n "$FIXED_SCRIPT" 2>/dev/null; then
                log_fail "Fixed script has syntax errors: $script"
                ((SYNTAX_ERRORS++)) || true
            fi
            rm -f "$FIXED_SCRIPT"
        fi
    done

    if [[ $SYNTAX_ERRORS -eq 0 ]]; then
        log_pass "All synthetic scripts pass syntax validation after fix"
    fi
fi

# =============================================================================
# Step 5: Test Fix Mode (Corpus)
# =============================================================================
if [[ "$TEST_FIX" == "true" ]] || [[ -d "testdata/corpus" && "$(ls -A testdata/corpus 2>/dev/null | grep -v .gitkeep)" ]]; then
    log_info "Testing fix mode..."

    # Clean output directory
    rm -rf testdata/fixed
    mkdir -p testdata/fixed

    # Fix sample file (expect exit 1 due to unfixable issues)
    FIX_OUTPUT=$(./macbash -o testdata/fixed/sample_fixed.sh testdata/sample_linux.sh 2>&1 || true)
    if echo "$FIX_OUTPUT" | grep -q "fixes applied\|fixes,"; then
        log_pass "Fix mode works on sample file"
    else
        log_fail "Fix mode failed on sample file"
    fi

    # Fix corpus if it exists and has files
    CORPUS_FILES=$(find testdata/corpus -name "*.sh" -type f 2>/dev/null | head -1)
    if [[ -n "$CORPUS_FILES" ]]; then
        log_info "Fixing corpus files..."
        CORPUS_COUNT=$(find testdata/corpus -name "*.sh" -type f | wc -l)

        if ./macbash -o testdata/fixed/ testdata/corpus/*.sh 2>&1; then
            FIXED_COUNT=$(find testdata/fixed -name "*.sh" -type f | wc -l)
            log_pass "Fixed $FIXED_COUNT corpus files"
        else
            # Exit code 1 means unfixable issues exist, which is expected
            FIXED_COUNT=$(find testdata/fixed -name "*.sh" -type f | wc -l)
            log_pass "Fixed $FIXED_COUNT corpus files (some unfixable issues)"
        fi
    else
        log_warn "No corpus files found - add scripts to testdata/corpus/ for more thorough testing"
    fi
fi

# =============================================================================
# Step 6: Bash 3.2 Syntax Validation (Docker)
# =============================================================================
if [[ "$QUICK_MODE" == "false" ]]; then
    log_info "Validating bash 3.2 syntax (Docker)..."

    # Check Docker is available
    if ! command -v docker &> /dev/null; then
        log_warn "Docker not available - skipping bash 3.2 validation"
    else
        # Pull bash 3.2 image if needed
        if ! docker image inspect bash:3.2 &> /dev/null; then
            log_info "Pulling bash:3.2 image..."
            docker pull bash:3.2
        fi

        # Test fixed files if they exist
        if [[ -d "testdata/fixed" ]] && [[ "$(ls -A testdata/fixed/*.sh 2>/dev/null)" ]]; then
            log_info "Checking fixed scripts with bash 3.2..."

            SYNTAX_ERRORS=0
            for script in testdata/fixed/*.sh; do
                if docker run --rm -v "$(pwd)/$script:/script.sh:ro" bash:3.2 bash -n /script.sh 2>/dev/null; then
                    : # OK
                else
                    log_fail "Syntax error in $script (bash 3.2)"
                    SYNTAX_ERRORS=$((SYNTAX_ERRORS + 1))
                fi
            done

            if [[ $SYNTAX_ERRORS -eq 0 ]]; then
                log_pass "All fixed scripts pass bash 3.2 syntax check"
            fi
        else
            log_warn "No fixed files to validate - run with --fix first"
        fi

        # Also check the sample output
        if [[ -f "testdata/fixed/sample_fixed.sh" ]]; then
            if docker run --rm -v "$(pwd)/testdata/fixed/sample_fixed.sh:/script.sh:ro" bash:3.2 bash -n /script.sh 2>/dev/null; then
                log_pass "sample_fixed.sh passes bash 3.2 syntax check"
            else
                log_fail "sample_fixed.sh has bash 3.2 syntax errors"
            fi
        fi
    fi
else
    log_warn "Skipping Docker tests (--quick mode)"
fi

# =============================================================================
# Step 7: Shellcheck (if available)
# =============================================================================
if command -v shellcheck &> /dev/null; then
    log_info "Running shellcheck on fixed scripts..."

    if [[ -d "testdata/fixed" ]] && [[ "$(ls -A testdata/fixed/*.sh 2>/dev/null)" ]]; then
        SHELLCHECK_ERRORS=0
        for script in testdata/fixed/*.sh; do
            if shellcheck -S warning "$script" > /dev/null 2>&1; then
                : # OK
            else
                log_warn "Shellcheck warnings in $script"
                # Don't count as failure - shellcheck is advisory
            fi
        done
        log_pass "Shellcheck completed"
    fi
else
    log_warn "shellcheck not installed - skipping"
fi

# =============================================================================
# Summary
# =============================================================================
echo ""
echo "========================================"
echo -e "Tests passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests failed: ${RED}$TESTS_FAILED${NC}"
echo "========================================"

if [[ $TESTS_FAILED -gt 0 ]]; then
    exit 1
fi

log_info "All local tests passed! Safe to push to CI."
