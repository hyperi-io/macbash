#!/bin/bash
# Sample script with GNU/Linux-specific constructs

# sed -i without backup (GNU-only)
sed -i 's/foo/bar/' file.txt

# grep -P Perl regex (GNU-only)
grep -P '\d+' numbers.txt

# readlink -f (GNU-only)
SCRIPT_DIR=$(readlink -f "$0")

# date -d epoch (GNU-only)
date -d @1609459200

# stat -c format (GNU-only)
stat -c '%s' file.txt

# xargs -r (GNU-only)
find . -name "*.tmp" | xargs -r rm

# declare -A associative array (bash 4+)
declare -A mymap

# case modification (bash 4+)
lower="${var,,}"

# |& pipe stderr (bash 4+)
command |& grep error

# echo -e (non-portable)
echo -e "line1\nline2"

# timeout command (GNU-only)
timeout 5 long_running_command
