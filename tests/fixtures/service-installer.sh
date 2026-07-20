#!/bin/bash
# service-installer.sh - Install and configure a system service
# Based on patterns from real installer scripts like pi-hole
# Contains GNU-specific features that need fixing for macOS

set -e

readonly INSTALL_DIR="/opt/myservice"
readonly CONFIG_DIR="/etc/myservice"
readonly LOG_DIR="/var/log/myservice"
readonly SERVICE_USER="myservice"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors
if [[ -t 1 ]];then
COL_NC='\e[0m'
COL_RED='\e[1;31m'
COL_GREEN='\e[1;32m'
COL_YELLOW='\e[1;33m'
else COL_NC='';COL_RED='';COL_GREEN='';COL_YELLOW='';fi

info() { echo -e "${COL_GREEN}[i]${COL_NC} $*"; }
warn() { echo -e "${COL_YELLOW}[!]${COL_NC} $*"; }
error() { echo -e "${COL_RED}[x]${COL_NC} $*" >&2; }
fatal() { error "$@"; exit 1; }

# Uses readlink -f
get_script_path() {
    local script_path
    script_path=$(readlink -f "${BASH_SOURCE[0]}")
    echo "$script_path"
}

check_root() {
    if [[ $EUID -ne 0 ]];then
        fatal "This script must be run as root"
    fi
}

# Uses stat -c to get file ownership
get_owner() {
    local target="$1"
    stat -c '%U' "$target" 2>/dev/null || echo "unknown"
}

get_group() {
    local target="$1"
    stat -c '%G' "$target" 2>/dev/null || echo "unknown"
}

get_perms() {
    local target="$1"
    stat -c '%a' "$target" 2>/dev/null || echo "000"
}

# Uses stat -c for size
get_size() {
    local file="$1"
    stat -c '%s' "$file" 2>/dev/null || echo "0"
}

# Uses date -d for relative dates
get_expiry_date() {
    local days="${1:-365}"
    date -d "+${days} days" '+%Y-%m-%d'
}

get_yesterday() {
    date -d "yesterday" '+%Y-%m-%d'
}

# Uses sed -i for in-place editing
update_config() {
    local key="$1"
    local value="$2"
    local config_file="${3:-$CONFIG_DIR/config.conf}"

    if grep -q "^${key}=" "$config_file" 2>/dev/null;then
        sed -i "s|^${key}=.*|${key}=${value}|" "$config_file"
    else
        echo "${key}=${value}" >> "$config_file"
    fi
}

# Multiple sed -i on same file
configure_logging() {
    local config="$CONFIG_DIR/logging.conf"

    [[ -f "$config" ]] || return 0

    # Fix log paths
    sed -i 's|/var/log/myservice.log|/var/log/myservice/service.log|g' "$config"
    sed -i 's|/var/log/myservice-debug.log|/var/log/myservice/debug.log|g' "$config"

    # Get log directory ownership
    local logusergroup
    logusergroup="$(stat -c '%U %G' /var/log)"

    # Update log rotation config
    sed -i "s/# su #/su ${logusergroup}/g;" "$config"
}

# Uses sed -i with different options
setup_cron() {
    local cronfile="/etc/cron.d/myservice"

    if [[ -f "$cronfile" ]];then
        # Randomize cron times to avoid thundering herd
        sed -i "s/59 1 /$((1 + RANDOM % 58)) $((3 + RANDOM % 2))/" "$cronfile"
        sed -i "s/59 17/$((1 + RANDOM % 58)) $((12 + RANDOM % 8))/" "$cronfile"
    fi
}

# Uses grep -P for perl regex
validate_email() {
    local email="$1"
    if echo "$email" | grep -qP '^[\w.+-]+@[\w.-]+\.\w{2,}$';then
        return 0
    fi
    return 1
}

validate_ip() {
    local ip="$1"
    if echo "$ip" | grep -qP '^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$';then
        return 0
    fi
    return 1
}

# Parse config with grep -P
parse_config_value() {
    local key="$1"
    local file="$2"
    grep -oP "^${key}=\K.*" "$file" 2>/dev/null || echo ""
}

#Creates directories and sets permissions
setup_directories() {
    mkdir -p "$INSTALL_DIR" "$CONFIG_DIR" "$LOG_DIR"

    # Create service user if needed
    if ! id "$SERVICE_USER" &>/dev/null;then
        useradd -r -s /bin/false "$SERVICE_USER" || true
    fi

    chown -R "$SERVICE_USER:$SERVICE_USER" "$LOG_DIR"
    chmod 750 "$LOG_DIR"

    info "Directories created"
}

# Install binary with verification
install_binary() {
    local src="$1"
    local dest="$2"

    if [[ ! -f "$src" ]];then
        fatal "Source file not found: $src"
    fi

    # Get source info using stat -c
    local src_size src_perms
    src_size=$(stat -c '%s' "$src")
    src_perms=$(stat -c '%a' "$src")

    cp "$src" "$dest"
    chmod +x "$dest"

    # Verify copy
    local dest_size
    dest_size=$(stat -c '%s' "$dest")

    if [[ "$src_size" != "$dest_size" ]];then
        fatal "Binary copy failed: size mismatch"
    fi

    info "Installed binary: $dest ($dest_size bytes)"
}

# Create systemd service
create_service() {
    local service_file="/etc/systemd/system/myservice.service"

    cat > "$service_file" <<'SERVICEEOF'
[Unit]
Description=My Service
After=network.target

[Service]
Type=simple
User=myservice
ExecStart=/opt/myservice/bin/myservice
Restart=on-failure

[Install]
WantedBy=multi-user.target
SERVICEEOF

    systemctl daemon-reload
    systemctl enable myservice
}

# Backup existing installation
backup_existing() {
    if [[ -d "$INSTALL_DIR" ]];then
        local backup_name="myservice_backup_$(date -d 'now' '+%Y%m%d_%H%M%S').tar.gz"
        tar czf "/tmp/$backup_name" -C "$(dirname "$INSTALL_DIR")" "$(basename "$INSTALL_DIR")"
        info "Backed up to /tmp/$backup_name"
    fi
}

#check existing version
check_version() {
    local current_version=""
    if [[ -f "$INSTALL_DIR/VERSION" ]];then
        current_version=$(cat "$INSTALL_DIR/VERSION")
        local install_date
        install_date=$(stat -c '%y' "$INSTALL_DIR/VERSION" | cut -d' ' -f1)
        info "Current version: $current_version (installed: $install_date)"
    fi
}

# Cleanup old logs using date -d
cleanup_old_logs() {
    local days="${1:-30}"
    local cutoff_date
    cutoff_date=$(date -d "-${days} days" '+%Y%m%d')

    info "Cleaning logs older than $days days..."

    find "$LOG_DIR" -name "*.log.*" -type f | while read -r logfile;do
        # Extract date from filename if present
        local file_date
        file_date=$(basename "$logfile" | grep -oP '\d{8}' | head -1)
        if [[ -n "$file_date" && "$file_date" < "$cutoff_date" ]];then
            rm -f "$logfile"
        fi
    done
}

show_completion() {
    local script_path
    script_path=$(readlink -f "$0")

    echo ""
    info "Installation complete!"
    echo ""
    echo "Configuration: $CONFIG_DIR/config.conf"
    echo "Logs: $LOG_DIR/"
    echo "Binary: $INSTALL_DIR/bin/myservice"
    echo ""
    echo "Script location: $script_path"
    echo "Expiry date: $(get_expiry_date 365)"
}

main() {
    local action="${1:-install}"

    case "$action" in
        install)
            check_root
            check_version
            backup_existing
            setup_directories
            configure_logging
            setup_cron
            show_completion
            ;;
        uninstall)
            check_root
            rm -rf "$INSTALL_DIR" "$CONFIG_DIR"
            userdel "$SERVICE_USER" 2>/dev/null || true
            info "Uninstalled"
            ;;
        backup)
            backup_existing
            ;;
        cleanup)
            cleanup_old_logs "${2:-30}"
            ;;
        *)
            echo "Usage: $0 {install|uninstall|backup|cleanup [days]}"
            exit 1
            ;;
    esac
}

main "$@"
