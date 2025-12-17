#!/bin/bash
# stress-test-fixer.sh - Edge cases that could break the fixer
# This script contains patterns where naive regex replacement would break syntax

set -euo pipefail

# Pattern 1: sed -i inside strings and heredocs - fixer must NOT touch these
MESSAGE='To edit in place use: sed -i "pattern" file'
echo "$MESSAGE"

cat > /tmp/docs.md <<'DOCEOF'
# How to use sed
You can edit files in place with sed -i:
  sed -i 's/old/new/g' myfile.txt
The -i flag means in-place editing.
DOCEOF

# Pattern 2: grep -P in comments and strings - should NOT be changed
# This explains grep -P usage: grep -P '\d+' matches digits
HELP_TEXT="Use grep -P for perl regex: grep -P '\\d+' file"

# Pattern 3: Actual sed -i usage that SHOULD be fixed
CONFIG=/tmp/test_config.txt
echo "server=localhost" > "$CONFIG"
sed -i 's/localhost/127.0.0.1/g' "$CONFIG"

# Pattern 4: Multiple sed -i on same line (both should be fixed)
FILE1=/tmp/f1.txt
FILE2=/tmp/f2.txt
echo "test" > "$FILE1"
echo "test" > "$FILE2"
sed -i 's/a/b/g' "$FILE1"; sed -i 's/c/d/g' "$FILE2"

# Pattern 5: sed -i with backup extension already (should NOT add another)
sed -i.bak 's/old/new/g' "$CONFIG" 2>/dev/null || true

# Pattern 6: sed -i '' macOS style (should NOT be modified)
# sed -i '' 's/x/y/g' "$CONFIG"

# Pattern 7: Complex sed with -i in different positions
sed -i -e 's/foo/bar/g' -e 's/baz/qux/g' "$CONFIG"
sed -E -i 's/[0-9]+/NUM/g' "$CONFIG"

# Pattern 8: grep -P that must be fixed
if grep -qP '^\d{3}\.\d{3}$' "$CONFIG" 2>/dev/null; then
    echo "Found IP-like pattern"
fi

# Pattern 9: readlink -f in variable assignment vs in strings
SCRIPT_DIR=$(readlink -f "$(dirname "$0")")
echo "Use readlink -f like: RESULT=\$(readlink -f path)"

# Pattern 10: date -d in actual usage vs documentation
YESTERDAY=$(date -d "yesterday" +%Y-%m-%d)
echo "To get yesterday: date -d 'yesterday' +%Y-%m-%d"

# Pattern 11: stat -c format string
FILE_SIZE=$(stat -c %s "$CONFIG")
echo "File size: $FILE_SIZE bytes"

# Pattern 12: echo -e that should be replaced
echo -e "Line 1\nLine 2\nLine 3"

# Pattern 13: echo -e in single quotes (should still work)
echo -e 'Tab:\tNewline:\n'

# Pattern 14: Nested quotes - tricky for regex
MSG="He said \"hello\""
echo -e "$MSG\n"

# Pattern 15: declare -A (bash 4+ associative array)
declare -A METADATA
METADATA["name"]="test"
METADATA["version"]="1.0"
echo "Name: ${METADATA[name]}"

# Pattern 16: ${var,,} lowercase (bash 4+)
INPUT="HELLO WORLD"
echo "${INPUT,,}"

# Pattern 17: ${var^^} uppercase (bash 4+)
input="hello world"
echo "${input^^}"

# Pattern 18: mapfile/readarray (bash 4+)
mapfile -t LINES < "$CONFIG"
echo "Read ${#LINES[@]} lines"

# Pattern 19: |& pipe with stderr (bash 4+)
ls /nonexistent |& grep -q "No such" && echo "Expected error"

# Pattern 20: Negative array subscript (bash 4.3+)
ARR=(one two three four)
echo "Last: ${ARR[-1]}"
echo "Second last: ${ARR[-2]}"

# Pattern 21: xargs -r (GNU only)
echo "" | xargs -r echo "This won't print"

# Pattern 22: find -printf (GNU only)
find /tmp -maxdepth 1 -name "test_*" -printf "%f %s\n" 2>/dev/null | head -5

# Pattern 23: sort -V version sort (GNU only)
echo -e "1.2\n1.10\n1.1\n2.0" | sort -V

# Pattern 24: timeout command (GNU coreutils)
timeout 1 sleep 0.5 || true

# Pattern 25: mktemp with template in different position
TMPFILE=$(mktemp /tmp/test.XXXXXX)
echo "Created: $TMPFILE"
rm -f "$TMPFILE"

# Pattern 26: Multiple patterns on single line with complex quoting
grep -P '^\s*#' "$CONFIG" 2>/dev/null || sed -i "s/^/# /" "$CONFIG"

# Pattern 27: Command substitution with these commands
RESULT=$(sed -i 's/NUM/\d+/g' "$CONFIG" 2>&1 && echo "OK")

# Pattern 28: Here-string with problematic commands
grep -P '\d' <<< "test123"

# Pattern 29: Inside functions
my_func() {
    local cfg="$1"
    sed -i 's/enabled=false/enabled=true/' "$cfg"
    grep -P 'enabled=(true|false)' "$cfg"
}
my_func "$CONFIG"

# Pattern 30: In eval (very tricky)
CMD='sed -i "s/test/prod/g" /tmp/dummy.txt'
# eval "$CMD" # uncomment to actually run

# Cleanup
rm -f /tmp/test_config.txt /tmp/f1.txt /tmp/f2.txt /tmp/docs.md

echo "Stress test completed"
