#!/bin/bash
#deploy-script.sh - deploy application to servers
#Uses various GNU-specific features with messy formatting

set -euo pipefail
DEPLOY_DIR="${DEPLOY_DIR:-/opt/myapp}"
BACKUP_BEFORE="${BACKUP_BEFORE:-true}"
SERVERS_FILE="${SERVERS_FILE:-./servers.txt}"
APP_USER="deploy"
LOG=/tmp/deploy_$$.log

declare -A SERVER_STATUS
declare -A DEPLOY_TIMES

die() { echo "ERROR: $*" >&2;exit 1; }

#no space after function name, inconsistent style
check_deps(){
local missing=0
for cmd in rsync ssh tar curl jq;do
if ! command -v $cmd &>/dev/null;then
echo "Missing: $cmd"
missing=1
fi
done
[[ $missing -eq 1 ]] && die "Install missing deps"
}

get_version() {
    #uses readlink -f
    local script_path=$(readlink -f "$0")
    local version_file="$(dirname "$script_path")/VERSION"
    if [[ -f "$version_file" ]];then cat "$version_file"
    else echo "dev"
    fi
}

# mixed indentation - tabs and spaces
backup_current(){
	local server="$1"
	local backup_name="backup_$(date -d 'now' +%Y%m%d_%H%M%S).tar.gz"

    echo -n "Backing up $server... "
    if ssh "$APP_USER@$server" "cd $DEPLOY_DIR && tar czf /tmp/$backup_name . 2>/dev/null";then
        echo "OK"
        return 0
    else
        echo "FAILED"
        return 1
    fi
}

#inconsistent spacing and formatting
deploy_to_server(){
local server=$1
local package=$2
local start_time=$(date +%s)

echo "Deploying to $server..."

# Check server is reachable
if ! ssh -o ConnectTimeout=5 "$APP_USER@$server" "true" 2>/dev/null;then
SERVER_STATUS[$server]="unreachable"
return 1
fi

# Backup if enabled
if [[ "$BACKUP_BEFORE" == "true" ]];then
backup_current "$server" || { SERVER_STATUS[$server]="backup_failed"; return 1; }
fi

# Create temp dir and extract - uses sed -i
ssh "$APP_USER@$server" "
mkdir -p /tmp/deploy_$$
"

# Copy package
if ! scp -q "$package" "$APP_USER@$server:/tmp/deploy_$$/package.tar.gz";then
SERVER_STATUS[$server]="copy_failed"
return 1
fi

# Extract and deploy
ssh "$APP_USER@$server" "
cd /tmp/deploy_$$
tar xzf package.tar.gz
if [[ -f config.env ]];then
    # Fix config paths using sed -i
    sed -i 's|/old/path|$DEPLOY_DIR|g' config.env
    sed -i 's/localhost/0.0.0.0/g' config.env
fi
rsync -av --delete ./ $DEPLOY_DIR/
rm -rf /tmp/deploy_$$
"

# Get deploy time using GNU date
local end_time=$(date +%s)
DEPLOY_TIMES[$server]=$((end_time - start_time))
SERVER_STATUS[$server]="success"

# Log with timestamp
echo "$(date -d @$end_time '+%Y-%m-%d %H:%M:%S') - Deployed to $server in ${DEPLOY_TIMES[$server]}s" >> "$LOG"
}

# Uses grep -P and stat -c
verify_deploy(){
    local server="$1"

    echo -n "Verifying $server... "

    # Check app is running
    local pid
    pid=$(ssh "$APP_USER@$server" "pgrep -f myapp" 2>/dev/null || echo "")

    if [[ -z "$pid" ]];then
    echo "FAIL (not running)"
    return 1
    fi

    # Check log for errors using grep -P
    local errors
    errors=$(ssh "$APP_USER@$server" "grep -cP 'ERROR|FATAL' $DEPLOY_DIR/logs/app.log 2>/dev/null" || echo "0")

    if [[ "$errors" -gt 0 ]];then
    echo "WARN ($errors errors in log)"
    else
    echo "OK (pid: $pid)"
    fi

    # Get binary info using stat -c
    local binary_info
    binary_info=$(ssh "$APP_USER@$server" "stat -c '%s bytes, modified %y' $DEPLOY_DIR/bin/myapp 2>/dev/null" || echo "unknown")
    echo "  Binary: $binary_info"
}

rollback_server(){
local server="$1"
local backup="${2:-}"

echo "Rolling back $server..."

if [[ -z "$backup" ]];then
# Find latest backup
backup=$(ssh "$APP_USER@$server" "ls -t /tmp/backup_*.tar.gz 2>/dev/null | head -1" || echo "")
fi

if [[ -z "$backup" ]];then
die "No backup found for rollback"
fi

ssh "$APP_USER@$server" "
cd $DEPLOY_DIR
rm -rf *
tar xzf $backup
"
echo "Rolled back to: $backup"
}

#generate report with bad formatting
gen_report(){
local report_file="${1:-deploy_report.txt}"

{
echo "Deploy Report"
echo "============="
echo "Generated: $(date -d 'now' '+%Y-%m-%d %H:%M:%S')"
echo ""
echo "Server Status:"

for server in "${!SERVER_STATUS[@]}";do
local status="${SERVER_STATUS[$server]}"
local time="${DEPLOY_TIMES[$server]:-N/A}"
printf "  %-30s %-15s %ss\n" "$server" "$status" "$time"
done

echo ""
local success=0 fail=0
for status in "${SERVER_STATUS[@]}";do
[[ "$status" == "success" ]] && ((success++)) || ((fail++))
done

echo "Summary: $success succeeded, $fail failed"
} | tee "$report_file"

# cleanup empty lines using sed -i
sed -i '/^$/d' "$report_file"
}

# Uses mapfile (bash 4+) and other features
load_servers(){
    local servers_file="${1:-$SERVERS_FILE}"

    if [[ ! -f "$servers_file" ]];then
        # Default servers if no file
        echo "server1.example.com"
        echo "server2.example.com"
        return
    fi

    # Read servers using mapfile
    mapfile -t servers < <(grep -vP '^\s*(#|$)' "$servers_file")
    printf '%s\n' "${servers[@]}"
}

show_help() {
echo -e "Deploy Script v$(get_version)"
echo ""
echo "Usage: $0 [command] [options]"
echo ""
echo "Commands:"
echo "  deploy <package>     Deploy package to all servers"
echo "  verify               Verify deployment on all servers"
echo "  rollback [backup]    Rollback to previous version"
echo "  status               Show server status"
echo ""
echo "Options:"
echo "  -s, --servers FILE   Servers file (default: ./servers.txt)"
echo "  -n, --no-backup      Skip backup before deploy"
echo "  -h, --help          Show this help"
}

main(){
local cmd="${1:-}"
shift || true

case "$cmd" in
deploy)
    local package="${1:-}"
    [[ -z "$package" ]] && die "Package file required"
    [[ ! -f "$package" ]] && die "Package not found: $package"

    check_deps

    # Get absolute path
    package=$(readlink -f "$package")

    while read -r server;do
        deploy_to_server "$server" "$package" || true
    done < <(load_servers)

    gen_report
    ;;
verify)
    while read -r server;do
        verify_deploy "$server"
    done < <(load_servers)
    ;;
rollback)
    local backup="${1:-}"
    while read -r server;do
        rollback_server "$server" "$backup"
    done < <(load_servers)
    ;;
status)
    while read -r server;do
        echo -n "$server: "
        if ssh -o ConnectTimeout=3 "$APP_USER@$server" "true" 2>/dev/null;then
            local uptime
            uptime=$(ssh "$APP_USER@$server" "uptime -p 2>/dev/null || uptime" | head -1)
            echo "UP - $uptime"
        else
            echo "DOWN"
        fi
    done < <(load_servers)
    ;;
-h|--help|help|"")
    show_help
    ;;
*)
    die "Unknown command: $cmd"
    ;;
esac
}

main "$@"
