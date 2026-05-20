# Project:   ${PKG_NAME}
# File:      uninstall.ps1 (rendered)
# Purpose:   Windows PowerShell uninstaller — locates and removes ${PKG_BINARY}.exe
# Language:  PowerShell
#
# License:   Apache-2.0
# Copyright: (c) 2025-2026 HYPERI PTY LIMITED
#
# Status:    EXPERIMENTAL — best effort. Windows is not in the formal
#            CI test path. Use at your own risk; expect rough edges.
#
# Usage:
#   irm ${PKG_DOWNLOAD_BASE}/uninstall.ps1 | iex
#   & ([scriptblock]::Create((irm ${PKG_DOWNLOAD_BASE}/uninstall.ps1))) -All

[CmdletBinding()]
param(
    [switch]$All,
    [string]$Dir
)

$ErrorActionPreference = 'Stop'

$binary     = '${PKG_BINARY}'
$defaultDir = "${PKG_WINDOWS_DEFAULT_DIR}"

Write-Host "[experimental] Windows uninstall is not in the formal CI test path." -ForegroundColor Yellow

function Remove-One {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) { return $false }
    try {
        Remove-Item -LiteralPath $Path -Force
        Write-Host "Removed $Path"
        return $true
    } catch {
        Write-Warning "Failed to remove $Path : $_"
        return $false
    }
}

if ($Dir) {
    $target = Join-Path $Dir "$binary.exe"
    if (-not (Remove-One $target)) {
        Write-Host "$binary not present in $Dir"
    }
    return
}

if ($All) {
    $any = $false
    # Drain every copy on PATH
    foreach ($p in $env:Path -split ';') {
        if (-not $p) { continue }
        $candidate = Join-Path $p "$binary.exe"
        if (Remove-One $candidate) { $any = $true }
    }
    # Plus the default install dir if not on PATH
    $candidate = Join-Path $defaultDir "$binary.exe"
    if (Remove-One $candidate) { $any = $true }
    if (-not $any) {
        Write-Host "$binary: no installations found."
    }
    return
}

# Default: locate via Get-Command (PATH lookup) and remove that one
$cmd = Get-Command -Name $binary -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $cmd) {
    Write-Host "$binary: not found on PATH. Use -All to scan thoroughly."
    return
}
Remove-One $cmd.Source | Out-Null

# Final advisory
$leftover = Get-Command -Name $binary -ErrorAction SilentlyContinue | Select-Object -First 1
if ($leftover) {
    Write-Host ""
    Write-Host "Note: $binary is still on PATH at $($leftover.Source)."
    Write-Host "  Re-run with -All to remove every copy."
}
