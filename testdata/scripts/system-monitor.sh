#!/bin/bash
#system-monitor.sh - system resource monitoring with alerting
#messy real-world script with GNU-isms

INTERVAL=${INTERVAL:-60}
ALERT_EMAIL=${ALERT_EMAIL:-admin@example.com}
CPU_THRESH=80
MEM_THRESH=90
DISK_THRESH=85
LOGFILE=/var/log/sysmon.log

declare -A PREV_CPU_STATS
declare -A ALERTS_SENT

log(){ echo "$(date -d now '+%Y-%m-%d %H:%M:%S') $*" >> "$LOGFILE"; }

# badly formatted but works
get_cpu_usage(){
local idle_time user_time sys_time total
read -r _ user_time _ sys_time idle_time _ < /proc/stat
total=$((user_time+sys_time+idle_time))
if [[ -n "${PREV_CPU_STATS[total]:-}" ]];then
local dtotal=$((total-PREV_CPU_STATS[total]))
local didle=$((idle_time-PREV_CPU_STATS[idle]))
echo $((100*(dtotal-didle)/dtotal))
else
echo 0
fi
PREV_CPU_STATS[total]=$total
PREV_CPU_STATS[idle]=$idle_time
}

get_mem_usage(){
    local total free buffers cached
    while read -r key val _;do
        case "$key" in
            MemTotal:) total=$val;;
            MemFree:) free=$val;;
            Buffers:) buffers=$val;;
            Cached:) cached=$val;;
        esac
    done < /proc/meminfo
    local used=$((total-free-buffers-cached))
    echo $((100*used/total))
}

get_disk_usage(){
local mount="${1:-/}"
df -h "$mount" 2>/dev/null | awk 'NR==2{gsub(/%/,"");print $5}'
}

# uses stat -c and readlink -f
get_file_info(){
    local file="$1"
    local realpath
    realpath=$(readlink -f "$file")
    stat -c "size=%s mode=%a mtime=%Y" "$realpath"
}

# uses grep -P for perl regex
check_log_errors(){
    local logfile="${1:-/var/log/syslog}"
    local since="${2:-1 hour ago}"

    if [[ ! -f "$logfile" ]];then
        echo "0"
        return
    fi

    # Get timestamp for filtering
    local since_epoch
    since_epoch=$(date -d "$since" +%s)

    # Count errors with perl regex
    grep -cP '(error|critical|fatal|panic)' "$logfile" 2>/dev/null || echo "0"
}

# uses sed -i for config updates
update_config(){
local key="$1" val="$2" cfg="${3:-/etc/sysmon.conf}"
if grep -q "^${key}=" "$cfg" 2>/dev/null;then
sed -i "s/^${key}=.*/${key}=${val}/" "$cfg"
else
echo "${key}=${val}" >> "$cfg"
fi
}

send_alert(){
local subject="$1"
local body="$2"
local alert_key="${3:-default}"

# Rate limit - one alert per type per hour
local now=$(date +%s)
local last_sent="${ALERTS_SENT[$alert_key]:-0}"
if (( now - last_sent < 3600 ));then
log "Alert suppressed (rate limit): $subject"
return
fi

log "ALERT: $subject"
if command -v mail &>/dev/null;then
echo "$body" | mail -s "[SYSMON] $subject" "$ALERT_EMAIL"
fi
ALERTS_SENT[$alert_key]=$now
}

# uses date -d @epoch format
format_uptime(){
    local uptime_secs
    uptime_secs=$(cut -d. -f1 /proc/uptime)
    local start_epoch=$(($(date +%s)-uptime_secs))
    echo "Up since $(date -d @$start_epoch '+%Y-%m-%d %H:%M:%S') (${uptime_secs}s)"
}

# uses mapfile (bash 4+) and ${var,,} lowercase
list_services(){
    local pattern="${1:-}"
    local services=()

    if [[ -d /etc/systemd/system ]];then
        mapfile -t services < <(systemctl list-units --type=service --state=running --no-pager --no-legend 2>/dev/null | awk '{print $1}')
    fi

    if [[ -n "$pattern" ]];then
        local filtered=()
        for svc in "${services[@]}";do
            # lowercase comparison
            [[ "${svc,,}" == *"${pattern,,}"* ]] && filtered+=("$svc")
        done
        printf '%s\n' "${filtered[@]}"
    else
        printf '%s\n' "${services[@]}"
    fi
}

# Process check with complex patterns
check_process(){
    local name="$1"
    local pids
    pids=$(pgrep -f "$name" 2>/dev/null || echo "")

    if [[ -z "$pids" ]];then
        echo "not_running"
        return 1
    fi

    local count=0 total_mem=0
    for pid in $pids;do
        ((count++))
        # Uses /proc for memory - Linux specific
        if [[ -f "/proc/$pid/status" ]];then
            local rss
            rss=$(grep -P '^VmRSS:' "/proc/$pid/status" 2>/dev/null | awk '{print $2}')
            ((total_mem+=rss)) || true
        fi
    done

    echo "running:$count:$total_mem"
}

#generate html report - messy multiline strings
gen_report(){
local output="${1:-/tmp/sysmon_report.html}"
local cpu mem disk uptime_info

cpu=$(get_cpu_usage)
mem=$(get_mem_usage)
disk=$(get_disk_usage /)
uptime_info=$(format_uptime)

cat > "$output" <<REPORTEOF
<!DOCTYPE html>
<html>
<head><title>System Report</title></head>
<body>
<h1>System Monitor Report</h1>
<p>Generated: $(date -d 'now' '+%Y-%m-%d %H:%M:%S')</p>
<p>$uptime_info</p>
<table border="1">
<tr><th>Metric</th><th>Value</th><th>Status</th></tr>
<tr><td>CPU</td><td>${cpu}%</td><td>$([ "$cpu" -lt "$CPU_THRESH" ] && echo "OK" || echo "WARN")</td></tr>
<tr><td>Memory</td><td>${mem}%</td><td>$([ "$mem" -lt "$MEM_THRESH" ] && echo "OK" || echo "WARN")</td></tr>
<tr><td>Disk</td><td>${disk}%</td><td>$([ "${disk:-0}" -lt "$DISK_THRESH" ] && echo "OK" || echo "WARN")</td></tr>
</table>
</body>
</html>
REPORTEOF

# Clean report
sed -i 's/[[:space:]]*$//' "$output"
log "Report generated: $output"
}

do_check(){
    local cpu mem disk errors

    cpu=$(get_cpu_usage)
    mem=$(get_mem_usage)
    disk=$(get_disk_usage /)
    errors=$(check_log_errors /var/log/syslog "10 minutes ago")

    log "CPU: ${cpu}% MEM: ${mem}% DISK: ${disk}% ERRORS: $errors"

    # Alert on thresholds
    [[ "$cpu" -ge "$CPU_THRESH" ]] && send_alert "High CPU: ${cpu}%" "CPU usage is ${cpu}% (threshold: ${CPU_THRESH}%)" "cpu"
    [[ "$mem" -ge "$MEM_THRESH" ]] && send_alert "High Memory: ${mem}%" "Memory usage is ${mem}% (threshold: ${MEM_THRESH}%)" "mem"
    [[ "${disk:-0}" -ge "$DISK_THRESH" ]] && send_alert "High Disk: ${disk}%" "Disk usage is ${disk}% (threshold: ${DISK_THRESH}%)" "disk"
}

main(){
local cmd="${1:-monitor}"

case "$cmd" in
monitor)
    log "Starting system monitor (interval: ${INTERVAL}s)"
    while true;do
        do_check
        sleep "$INTERVAL"
    done
    ;;
once)
    do_check
    ;;
report)
    gen_report "${2:-/tmp/sysmon_report.html}"
    ;;
services)
    list_services "${2:-}"
    ;;
process)
    check_process "${2:-sysmon}"
    ;;
*)
    echo "Usage: $0 {monitor|once|report [file]|services [pattern]|process <name>}"
    exit 1
    ;;
esac
}

main "$@"
