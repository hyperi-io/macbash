#!/bin/bash
# log-analyzer.sh - Parse and analyze log files
# Uses: grep -P, sed -i, date -d @epoch, stat -c, mapfile

LOG_DIR="${LOG_DIR:-/var/log}"
OUTPUT_DIR="${OUTPUT_DIR:-./analysis}"
PATTERN_FILE="${PATTERN_FILE:-}"

# Bash 4+ associative arrays
declare -A ERROR_COUNTS
declare -A IP_HITS
declare -A HOURLY_STATS

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

usage() {
    cat <<EOF
Log Analyzer - Parse and analyze log files

Usage: $(basename "$0") [OPTIONS] <log_file>

Options:
    -o, --output DIR    Output directory (default: ./analysis)
    -p, --pattern FILE  Custom patterns file
    -t, --top N         Show top N results (default: 10)
    -f, --format FMT    Output format: text, json, csv
    -q, --quiet         Suppress progress output
    -h, --help          Show this help

Examples:
    $(basename "$0") /var/log/nginx/access.log
    $(basename "$0") -o ./reports -t 20 /var/log/auth.log
EOF
    exit 0
}

log_msg() {
    local color="$1"
    shift
    echo -e "${color}$*${NC}"
}

parse_timestamp() {
    local timestamp_str="$1"
    local format="${2:-}"

    # Try parsing different formats using GNU date -d
    if [[ -n "$format" ]]; then
        date -d "$timestamp_str" "+%s" 2>/dev/null
    elif [[ "$timestamp_str" =~ ^[0-9]+$ ]]; then
        echo "$timestamp_str"
    else
        # Try common formats
        date -d "$timestamp_str" "+%s" 2>/dev/null || \
        date -d "$(echo "$timestamp_str" | sed 's/\// /g')" "+%s" 2>/dev/null || \
        echo "0"
    fi
}

epoch_to_human() {
    local epoch="$1"
    # GNU date -d @epoch syntax
    date -d "@$epoch" "+%Y-%m-%d %H:%M:%S" 2>/dev/null || echo "unknown"
}

extract_ips() {
    local file="$1"

    # Use grep -P for Perl regex to match IPs
    grep -oP '\b(?:\d{1,3}\.){3}\d{1,3}\b' "$file" 2>/dev/null | sort | uniq -c | sort -rn
}

extract_errors() {
    local file="$1"

    # Match error patterns with Perl regex
    grep -iP '(error|fail|exception|critical|fatal)' "$file" 2>/dev/null | head -100
}

analyze_access_log() {
    local file="$1"
    local line_count=0
    local error_count=0

    log_msg "$GREEN" "Analyzing access log: $file"

    # Read file into array using mapfile (bash 4+)
    mapfile -t LOG_LINES < "$file"

    for line in "${LOG_LINES[@]}"; do
        ((line_count++)) || true

        # Extract IP using Perl regex
        local ip
        ip=$(echo "$line" | grep -oP '^(?:\d{1,3}\.){3}\d{1,3}' 2>/dev/null || echo "unknown")

        if [[ "$ip" != "unknown" ]]; then
            ((IP_HITS["$ip"]++)) || IP_HITS["$ip"]=1
        fi

        # Extract hour
        local hour
        hour=$(echo "$line" | grep -oP '\d{2}(?=:\d{2}:\d{2})' 2>/dev/null | head -1)
        if [[ -n "$hour" ]]; then
            ((HOURLY_STATS["$hour"]++)) || HOURLY_STATS["$hour"]=1
        fi

        # Check for errors
        if echo "$line" | grep -qP '"[^"]*"\s+[45]\d{2}\s' 2>/dev/null; then
            ((error_count++)) || true
        fi

        # Progress indicator
        if ((line_count % 1000 == 0)); then
            echo -ne "\rProcessed $line_count lines..."
        fi
    done

    echo ""
    log_msg "$GREEN" "Total lines: $line_count, Errors: $error_count"
}

analyze_auth_log() {
    local file="$1"

    log_msg "$GREEN" "Analyzing auth log: $file"

    local failed_logins successful_logins
    failed_logins=$(grep -cP 'Failed password|authentication failure' "$file" 2>/dev/null || echo "0")
    successful_logins=$(grep -cP 'Accepted (password|publickey)' "$file" 2>/dev/null || echo "0")

    # Extract failed login IPs
    local failed_ips
    failed_ips=$(grep -P 'Failed password' "$file" 2>/dev/null | \
                 grep -oP 'from \K(?:\d{1,3}\.){3}\d{1,3}' | \
                 sort | uniq -c | sort -rn | head -10)

    echo "Failed logins: $failed_logins"
    echo "Successful logins: $successful_logins"
    echo ""
    echo "Top failed login sources:"
    echo "$failed_ips"
}

generate_report() {
    local output_file="$1"
    local format="${2:-text}"

    log_msg "$GREEN" "Generating report: $output_file ($format)"

    mkdir -p "$(dirname "$output_file")"

    case "$format" in
        json)
            {
                echo "{"
                echo '  "generated": "'$(date -d 'now' '+%Y-%m-%dT%H:%M:%S')'",'
                echo '  "ip_stats": {'

                local first=true
                for ip in "${!IP_HITS[@]}"; do
                    $first || echo ","
                    first=false
                    echo -n "    \"$ip\": ${IP_HITS[$ip]}"
                done

                echo ""
                echo "  },"
                echo '  "hourly_stats": {'

                first=true
                for hour in $(echo "${!HOURLY_STATS[@]}" | tr ' ' '\n' | sort); do
                    $first || echo ","
                    first=false
                    echo -n "    \"$hour\": ${HOURLY_STATS[$hour]}"
                done

                echo ""
                echo "  }"
                echo "}"
            } > "$output_file"
            ;;

        csv)
            {
                echo "type,key,value"
                for ip in "${!IP_HITS[@]}"; do
                    echo "ip,$ip,${IP_HITS[$ip]}"
                done
                for hour in "${!HOURLY_STATS[@]}"; do
                    echo "hour,$hour,${HOURLY_STATS[$hour]}"
                done
            } > "$output_file"
            ;;

        *)  # text
            {
                echo "Log Analysis Report"
                echo "==================="
                echo "Generated: $(date -d 'now' '+%Y-%m-%d %H:%M:%S')"
                echo ""

                echo "Top IPs:"
                for ip in "${!IP_HITS[@]}"; do
                    printf "  %-20s %d\n" "$ip" "${IP_HITS[$ip]}"
                done | sort -t$'\t' -k2 -rn | head -"${TOP_N:-10}"

                echo ""
                echo "Hourly Distribution:"
                for hour in $(echo "${!HOURLY_STATS[@]}" | tr ' ' '\n' | sort); do
                    local count="${HOURLY_STATS[$hour]}"
                    local bar
                    bar=$(printf '%*s' "$((count / 10))" | tr ' ' '#')
                    printf "  %s:00 %5d %s\n" "$hour" "$count" "$bar"
                done
            } > "$output_file"
            ;;
    esac

    # Clean up report file
    sed -i '/^[[:space:]]*$/d' "$output_file"

    local file_size
    file_size=$(stat -c %s "$output_file")
    log_msg "$GREEN" "Report written: $output_file ($(numfmt --to=iec "$file_size" 2>/dev/null || echo "${file_size}B"))"
}

cleanup_old_reports() {
    local days="${1:-7}"
    local deleted=0

    log_msg "$YELLOW" "Cleaning reports older than $days days..."

    while IFS= read -r -d '' file; do
        local file_mtime
        file_mtime=$(stat -c %Y "$file")
        local current
        current=$(date +%s)
        local age_days=$(( (current - file_mtime) / 86400 ))

        if [[ $age_days -gt $days ]]; then
            rm -f "$file"
            ((deleted++)) || true
        fi
    done < <(find "$OUTPUT_DIR" -name "*.txt" -o -name "*.json" -o -name "*.csv" -print0 2>/dev/null)

    log_msg "$GREEN" "Cleaned up $deleted old reports"
}

main() {
    local log_file=""
    local format="text"
    local top_n=10
    local quiet=false

    while [[ $# -gt 0 ]]; do
        case "$1" in
            -o|--output)
                OUTPUT_DIR="$2"
                shift 2
                ;;
            -p|--pattern)
                PATTERN_FILE="$2"
                shift 2
                ;;
            -t|--top)
                top_n="$2"
                shift 2
                ;;
            -f|--format)
                format="${2,,}"  # lowercase using bash 4+ syntax
                shift 2
                ;;
            -q|--quiet)
                quiet=true
                shift
                ;;
            -h|--help)
                usage
                ;;
            *)
                log_file="$1"
                shift
                ;;
        esac
    done

    if [[ -z "$log_file" ]]; then
        log_msg "$RED" "Error: No log file specified"
        usage
    fi

    if [[ ! -f "$log_file" ]]; then
        log_msg "$RED" "Error: File not found: $log_file"
        exit 1
    fi

    TOP_N="$top_n"
    mkdir -p "$OUTPUT_DIR"

    # Determine log type and analyze
    local basename_log
    basename_log=$(basename "$log_file")

    if echo "$basename_log" | grep -qP '(access|nginx|apache)'; then
        analyze_access_log "$log_file"
    elif echo "$basename_log" | grep -qP '(auth|secure|sshd)'; then
        analyze_auth_log "$log_file"
    else
        # Generic analysis
        analyze_access_log "$log_file"
    fi

    # Generate report
    local report_name
    report_name="report_${basename_log%.*}_$(date -d 'now' '+%Y%m%d_%H%M%S')"

    case "$format" in
        json) report_name="${report_name}.json" ;;
        csv)  report_name="${report_name}.csv" ;;
        *)    report_name="${report_name}.txt" ;;
    esac

    generate_report "$OUTPUT_DIR/$report_name" "$format"

    # Cleanup old reports
    cleanup_old_reports 7
}

main "$@"
