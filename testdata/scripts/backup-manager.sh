#!/bin/bash
# backup-manager.sh - A backup rotation script with GNU-specific features
# This script manages backup rotation with date-based naming and pruning
# Uses: sed -i, date -d, readlink -f, grep -P, stat -c

set -euo pipefail

BACKUP_DIR="${BACKUP_DIR:-/var/backups}"
RETENTION_DAYS="${RETENTION_DAYS:-30}"
CONFIG_FILE="${CONFIG_FILE:-/etc/backup-manager.conf}"
LOG_FILE="/var/log/backup-manager.log"
DRY_RUN="${DRY_RUN:-false}"

declare -A BACKUP_TARGETS

log() {
    local level="$1"
    shift
    local msg="$*"
    local timestamp
    timestamp=$(date -d "now" "+%Y-%m-%d %H:%M:%S")
    echo -e "[$timestamp] [$level] $msg" | tee -a "$LOG_FILE"
}

debug() { [[ "${DEBUG:-false}" == "true" ]] && log "DEBUG" "$@" || true; }
info() { log "INFO" "$@"; }
warn() { log "WARN" "$@"; }
error() { log "ERROR" "$@"; }

load_config() {
    local config_path
    config_path=$(readlink -f "$CONFIG_FILE")

    if [[ ! -f "$config_path" ]]; then
        warn "Config file not found: $config_path"
        return 1
    fi

    # Parse config using grep -P for Perl regex
    while IFS='=' read -r key value; do
        [[ -z "$key" || "$key" =~ ^[[:space:]]*# ]] && continue
        key=$(echo "$key" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
        value=$(echo "$value" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')

        if echo "$key" | grep -qP '^backup_target_\d+$'; then
            local target_name
            target_name=$(echo "$key" | grep -oP '\d+$')
            BACKUP_TARGETS["$target_name"]="$value"
            debug "Loaded backup target: $target_name -> $value"
        fi
    done < "$config_path"

    info "Loaded ${#BACKUP_TARGETS[@]} backup targets"
}

get_backup_age_days() {
    local file="$1"
    local file_mtime
    local current_time
    local age_seconds

    # GNU stat -c for format string
    file_mtime=$(stat -c %Y "$file")
    current_time=$(date +%s)
    age_seconds=$((current_time - file_mtime))
    echo $((age_seconds / 86400))
}

format_size() {
    local bytes="$1"
    local size_str

    if [[ $bytes -ge 1073741824 ]]; then
        size_str=$(echo "scale=2; $bytes / 1073741824" | bc)
        echo "${size_str}G"
    elif [[ $bytes -ge 1048576 ]]; then
        size_str=$(echo "scale=2; $bytes / 1048576" | bc)
        echo "${size_str}M"
    elif [[ $bytes -ge 1024 ]]; then
        size_str=$(echo "scale=2; $bytes / 1024" | bc)
        echo "${size_str}K"
    else
        echo "${bytes}B"
    fi
}

create_backup() {
    local target_path="$1"
    local target_name
    local backup_name
    local backup_path
    local real_path

    real_path=$(readlink -f "$target_path")
    target_name=$(basename "$real_path")

    # Create backup name with GNU date -d
    backup_name="${target_name}_$(date -d 'now' '+%Y%m%d_%H%M%S').tar.gz"
    backup_path="$BACKUP_DIR/$backup_name"

    info "Creating backup: $backup_path"

    if [[ "$DRY_RUN" == "true" ]]; then
        debug "[DRY RUN] Would create: $backup_path"
        return 0
    fi

    if [[ -d "$real_path" ]]; then
        tar -czf "$backup_path" -C "$(dirname "$real_path")" "$(basename "$real_path")"
    elif [[ -f "$real_path" ]]; then
        tar -czf "$backup_path" -C "$(dirname "$real_path")" "$(basename "$real_path")"
    else
        error "Target does not exist: $real_path"
        return 1
    fi

    local backup_size
    backup_size=$(stat -c %s "$backup_path")
    info "Backup created: $backup_path ($(format_size "$backup_size"))"

    # Update manifest using sed -i
    local manifest="$BACKUP_DIR/manifest.txt"
    echo "$backup_name|$(date -d 'now' '+%Y-%m-%d %H:%M:%S')|$backup_size" >> "$manifest"
    sed -i '/^$/d' "$manifest"  # Remove empty lines
}

prune_old_backups() {
    local deleted_count=0
    local deleted_size=0

    info "Pruning backups older than $RETENTION_DAYS days..."

    shopt -s nullglob
    for backup_file in "$BACKUP_DIR"/*.tar.gz; do
        local age_days
        age_days=$(get_backup_age_days "$backup_file")

        if [[ $age_days -gt $RETENTION_DAYS ]]; then
            local file_size
            file_size=$(stat -c %s "$backup_file")

            if [[ "$DRY_RUN" == "true" ]]; then
                debug "[DRY RUN] Would delete: $backup_file (age: ${age_days}d)"
            else
                info "Deleting old backup: $backup_file (age: ${age_days}d)"
                rm -f "$backup_file"

                # Update manifest - remove entry
                local manifest="$BACKUP_DIR/manifest.txt"
                if [[ -f "$manifest" ]]; then
                    local basename_backup
                    basename_backup=$(basename "$backup_file")
                    sed -i "/^${basename_backup}|/d" "$manifest"
                fi
            fi

            ((deleted_count++)) || true
            ((deleted_size += file_size)) || true
        fi
    done
    shopt -u nullglob

    if [[ $deleted_count -gt 0 ]]; then
        info "Pruned $deleted_count backups ($(format_size "$deleted_size") freed)"
    else
        info "No backups to prune"
    fi
}

generate_report() {
    local report_file="$BACKUP_DIR/report_$(date -d 'now' '+%Y%m%d').txt"
    local total_size=0
    local backup_count=0

    info "Generating backup report..."

    {
        echo "Backup Report - Generated $(date -d 'now' '+%Y-%m-%d %H:%M:%S')"
        echo "=============================================="
        echo ""
        printf "%-50s %10s %10s\n" "Backup File" "Size" "Age (days)"
        echo "--------------------------------------------------------------"

        shopt -s nullglob
        for backup_file in "$BACKUP_DIR"/*.tar.gz; do
            local file_size file_age basename_file
            file_size=$(stat -c %s "$backup_file")
            file_age=$(get_backup_age_days "$backup_file")
            basename_file=$(basename "$backup_file")

            printf "%-50s %10s %10s\n" "$basename_file" "$(format_size "$file_size")" "$file_age"

            ((total_size += file_size)) || true
            ((backup_count++)) || true
        done
        shopt -u nullglob

        echo "--------------------------------------------------------------"
        printf "%-50s %10s %10s\n" "TOTAL: $backup_count backups" "$(format_size "$total_size")" "-"
    } > "$report_file"

    info "Report saved: $report_file"
}

verify_backup() {
    local backup_file="$1"

    if [[ ! -f "$backup_file" ]]; then
        error "Backup file not found: $backup_file"
        return 1
    fi

    info "Verifying backup: $backup_file"

    if tar -tzf "$backup_file" > /dev/null 2>&1; then
        local file_count
        file_count=$(tar -tzf "$backup_file" | wc -l)
        info "Backup verified: $file_count files"
        return 0
    else
        error "Backup verification failed: $backup_file"
        return 1
    fi
}

main() {
    local action="${1:-backup}"

    mkdir -p "$BACKUP_DIR"

    case "$action" in
        backup)
            if ! load_config 2>/dev/null; then
                # Default target if no config
                BACKUP_TARGETS["default"]="/home"
            fi

            for target_name in "${!BACKUP_TARGETS[@]}"; do
                local target_path="${BACKUP_TARGETS[$target_name]}"
                create_backup "$target_path"
            done
            ;;
        prune)
            prune_old_backups
            ;;
        report)
            generate_report
            ;;
        verify)
            local backup_file="${2:-}"
            if [[ -z "$backup_file" ]]; then
                error "Please specify a backup file to verify"
                exit 1
            fi
            verify_backup "$backup_file"
            ;;
        all)
            load_config 2>/dev/null || BACKUP_TARGETS["default"]="/home"
            for target_name in "${!BACKUP_TARGETS[@]}"; do
                create_backup "${BACKUP_TARGETS[$target_name]}"
            done
            prune_old_backups
            generate_report
            ;;
        *)
            echo "Usage: $0 {backup|prune|report|verify <file>|all}"
            exit 1
            ;;
    esac
}

main "$@"
